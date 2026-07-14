[CmdletBinding()]
param(
    [switch]$ValidateOnly,
    [switch]$SkipLaunch
)

if ($env:AGENTIC_COUNCIL_VALIDATE_ONLY -eq '1') {
    $ValidateOnly = $true
}
if ($env:AGENTIC_COUNCIL_SKIP_LAUNCH -eq '1') {
    $SkipLaunch = $true
}

Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# Windows PowerShell 5.1 can otherwise negotiate an obsolete TLS version.
[Net.ServicePointManager]::SecurityProtocol =
    [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$script:StepNumber = 0
$script:TotalSteps = 6
$script:TempRoot = Join-Path ([IO.Path]::GetTempPath()) ("AgenticCouncilInstaller-{0}" -f $PID)
$ProjectRoot = Split-Path -Parent $PSCommandPath
$ToolsRoot = Join-Path $env:LOCALAPPDATA 'AgenticCouncil\tools'
$RustToolchain = '1.97.0'

function Write-Step {
    param([string]$Message)
    $script:StepNumber++
    Write-Host ''
    Write-Host ("[STEP {0}/{1}] {2}" -f $script:StepNumber, $script:TotalSteps, $Message) -ForegroundColor Cyan
}

function Write-Ok {
    param([string]$Message)
    Write-Host ("  [OK] {0}" -f $Message) -ForegroundColor Green
}

function Write-Info {
    param([string]$Message)
    Write-Host ("  [INFO] {0}" -f $Message)
}

function Write-WarningMessage {
    param([string]$Message)
    Write-Host ("  [WARNING] {0}" -f $Message) -ForegroundColor Yellow
}

function Invoke-Download {
    param(
        [Parameter(Mandatory = $true)][string]$Uri,
        [Parameter(Mandatory = $true)][string]$Destination
    )

    $lastError = $null
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        try {
            Invoke-WebRequest -UseBasicParsing -Uri $Uri -OutFile $Destination
            return
        } catch {
            $lastError = $_
            if ($attempt -lt 3) {
                Write-Info ("Download attempt {0} failed; retrying..." -f $attempt)
                Start-Sleep -Seconds 2
            }
        }
    }

    throw ("Could not download {0}. Check the internet connection. {1}" -f $Uri, $lastError.Exception.Message)
}

function Get-NativeArchitecture {
    $architecture = $env:PROCESSOR_ARCHITEW6432
    if ([string]::IsNullOrWhiteSpace($architecture)) {
        $architecture = $env:PROCESSOR_ARCHITECTURE
    }

    switch ($architecture.ToUpperInvariant()) {
        'AMD64' { return @{ Node = 'x64'; Rust = 'x86_64-pc-windows-msvc' } }
        'ARM64' { return @{ Node = 'arm64'; Rust = 'aarch64-pc-windows-msvc' } }
        'X86'   { return @{ Node = 'x86'; Rust = 'i686-pc-windows-msvc' } }
        default { throw "Unsupported Windows processor architecture: $architecture" }
    }
}

function Get-NodeVersion {
    param([Parameter(Mandatory = $true)][string]$NodePath)

    try {
        $value = (& $NodePath --version 2>$null)
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($value)) {
            return $null
        }
        return [version]$value.Trim().TrimStart('v')
    } catch {
        return $null
    }
}

function Test-NodeVersion {
    param([AllowNull()][version]$Version)
    return $null -ne $Version -and $Version -ge [version]'22.12.0'
}

function Find-CompatibleNode {
    $systemNode = Get-Command node.exe -ErrorAction SilentlyContinue
    if ($null -ne $systemNode) {
        $version = Get-NodeVersion -NodePath $systemNode.Source
        if (Test-NodeVersion -Version $version) {
            return @{ Path = $systemNode.Source; Version = $version }
        }
    }

    if (Test-Path -LiteralPath $ToolsRoot) {
        $localNodes = Get-ChildItem -LiteralPath $ToolsRoot -Directory -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -like 'node-v*-win-*' }
        foreach ($directory in $localNodes) {
            $nodePath = Join-Path $directory.FullName 'node.exe'
            if (Test-Path -LiteralPath $nodePath) {
                $version = Get-NodeVersion -NodePath $nodePath
                if (Test-NodeVersion -Version $version) {
                    return @{ Path = $nodePath; Version = $version }
                }
            }
        }
    }

    return $null
}

function Install-PortableNode {
    param([string]$NodeArchitecture)

    Write-Info 'Downloading a private Node.js LTS copy for Agentic Council...'
    $releases = Invoke-RestMethod -UseBasicParsing -Uri 'https://nodejs.org/dist/index.json'
    $release = $releases |
        Where-Object { $_.version -like 'v22.*' -and $_.lts -ne $false } |
        Select-Object -First 1
    if ($null -eq $release) {
        throw 'The Node.js download service did not return a Node 22 LTS release.'
    }

    $fileKey = "win-$NodeArchitecture-zip"
    if ($release.files -notcontains $fileKey) {
        throw ("Node.js {0} does not provide {1}." -f $release.version, $fileKey)
    }

    $fileName = "node-{0}-win-{1}.zip" -f $release.version, $NodeArchitecture
    $baseUri = "https://nodejs.org/dist/{0}" -f $release.version
    $archivePath = Join-Path $script:TempRoot $fileName
    Invoke-Download -Uri "$baseUri/$fileName" -Destination $archivePath

    $checksums = (Invoke-WebRequest -UseBasicParsing -Uri "$baseUri/SHASUMS256.txt").Content
    $match = [regex]::Match([string]$checksums, ("(?m)^([a-fA-F0-9]{{64}})\s+{0}\s*$" -f [regex]::Escape($fileName)))
    if (-not $match.Success) {
        throw "Could not find the official checksum for $fileName."
    }
    $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash
    if ($actualHash -ne $match.Groups[1].Value) {
        throw "The Node.js download failed its security checksum."
    }

    New-Item -ItemType Directory -Force -Path $ToolsRoot | Out-Null
    Expand-Archive -LiteralPath $archivePath -DestinationPath $ToolsRoot -Force
    $nodePath = Join-Path $ToolsRoot (("node-{0}-win-{1}\node.exe" -f $release.version, $NodeArchitecture))
    $version = Get-NodeVersion -NodePath $nodePath
    if (-not (Test-NodeVersion -Version $version)) {
        throw 'Node.js was downloaded but could not be started.'
    }
    return @{ Path = $nodePath; Version = $version }
}

function Install-Rustup {
    param([string]$RustTarget)

    Write-Info 'Downloading rustup from the official Rust servers...'
    $baseUri = "https://static.rust-lang.org/rustup/dist/$RustTarget"
    $installerPath = Join-Path $script:TempRoot 'rustup-init.exe'
    $checksumPath = Join-Path $script:TempRoot 'rustup-init.exe.sha256'
    $expectedHash = $null
    $actualHash = $null

    # Download both files for each attempt. Rust's CDN can briefly serve the
    # installer and checksum from different cache generations during an update.
    # Reading the checksum from disk also avoids Windows PowerShell 5.1 treating
    # Invoke-WebRequest.Content as bytes for some response content types.
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        Invoke-Download -Uri "$baseUri/rustup-init.exe" -Destination $installerPath
        Invoke-Download -Uri "$baseUri/rustup-init.exe.sha256" -Destination $checksumPath

        $checksumText = Get-Content -Raw -LiteralPath $checksumPath
        $checksumMatch = [regex]::Match([string]$checksumText, '(?i)(?<![a-f0-9])[a-f0-9]{64}(?![a-f0-9])')
        if (-not $checksumMatch.Success) {
            if ($attempt -lt 3) {
                Write-Info ("Rust checksum response was invalid on attempt {0}; retrying both files..." -f $attempt)
                continue
            }
            throw 'Rust did not return a valid SHA-256 checksum. A proxy or security filter may be replacing the response from static.rust-lang.org.'
        }

        $expectedHash = $checksumMatch.Value
        $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installerPath).Hash
        if ($actualHash -ieq $expectedHash) {
            break
        }
        if ($attempt -lt 3) {
            Write-Info ("Rust installer and checksum did not match on attempt {0}; retrying both files..." -f $attempt)
        }
    }

    if ($actualHash -ine $expectedHash) {
        throw 'The Rust installer did not match Rust''s published checksum after 3 attempts. A proxy, VPN, antivirus web shield, or filtered network may be changing the download. Allow static.rust-lang.org or try another network, then run install.bat again.'
    }

    $process = Start-Process -FilePath $installerPath -ArgumentList @(
        '-y', '--profile', 'minimal', '--default-toolchain', 'none'
    ) -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw ("The Rust installer exited with code {0}." -f $process.ExitCode)
    }
}

function Get-VCToolsInstallation {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere)) {
        return $null
    }

    $installation = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null) |
        Select-Object -First 1
    if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($installation)) {
        return $installation.Trim()
    }
    return $null
}

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Install-VCTools {
    Write-Info 'Downloading Microsoft C++ Build Tools...'
    Write-Info 'This is the largest download. Windows may ask for permission; choose Yes.'
    $installerPath = Join-Path $script:TempRoot 'vs_BuildTools.exe'
    Invoke-Download -Uri 'https://aka.ms/vs/17/release/vs_BuildTools.exe' -Destination $installerPath

    $signature = Get-AuthenticodeSignature -LiteralPath $installerPath
    if ($signature.Status -ne 'Valid') {
        throw 'The Microsoft Build Tools installer does not have a valid digital signature.'
    }

    $arguments = @(
        '--passive', '--wait', '--norestart', '--nocache',
        '--add', 'Microsoft.VisualStudio.Workload.VCTools', '--includeRecommended'
    )
    if (Test-Administrator) {
        $process = Start-Process -FilePath $installerPath -ArgumentList $arguments -Wait -PassThru
    } else {
        $process = Start-Process -FilePath $installerPath -ArgumentList $arguments -Verb RunAs -Wait -PassThru
    }

    if ($process.ExitCode -notin @(0, 3010)) {
        throw ("Microsoft C++ Build Tools exited with code {0}." -f $process.ExitCode)
    }
    if ($process.ExitCode -eq 3010) {
        Write-WarningMessage 'Windows recommends a restart, but setup will try to continue.'
    }
    if ($null -eq (Get-VCToolsInstallation)) {
        throw 'C++ Build Tools finished installing, but the required compiler was not detected. Restart Windows and run install.bat again.'
    }
}

function Get-WebView2Version {
    $clientId = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    $registryPaths = @(
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$clientId",
        "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$clientId",
        "HKCU:\Software\Microsoft\EdgeUpdate\Clients\$clientId"
    )
    foreach ($path in $registryPaths) {
        try {
            $version = (Get-ItemProperty -LiteralPath $path -Name 'pv' -ErrorAction Stop).pv
            if (-not [string]::IsNullOrWhiteSpace($version) -and $version -ne '0.0.0.0') {
                return $version
            }
        } catch {
            # Keep checking the other supported registry locations.
        }
    }
    return $null
}

function Install-WebView2 {
    Write-Info 'Downloading the Microsoft WebView2 Evergreen bootstrapper...'
    $installerPath = Join-Path $script:TempRoot 'MicrosoftEdgeWebview2Setup.exe'
    Invoke-Download -Uri 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -Destination $installerPath
    $signature = Get-AuthenticodeSignature -LiteralPath $installerPath
    if ($signature.Status -ne 'Valid') {
        throw 'The Microsoft WebView2 installer does not have a valid digital signature.'
    }

    $process = Start-Process -FilePath $installerPath -ArgumentList @('/silent', '/install') -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw ("The WebView2 installer exited with code {0}." -f $process.ExitCode)
    }
    for ($attempt = 0; $attempt -lt 10; $attempt++) {
        $version = Get-WebView2Version
        if ($null -ne $version) {
            return $version
        }
        Start-Sleep -Seconds 1
    }
    throw 'WebView2 installation finished, but its runtime was not detected.'
}

function Assert-CompleteProject {
    $requiredFiles = @(
        'package.json',
        'rust-toolchain.toml',
        'src-tauri\Cargo.toml'
    )
    $missingFiles = @()
    foreach ($relativePath in $requiredFiles) {
        if (-not (Test-Path -LiteralPath (Join-Path $ProjectRoot $relativePath) -PathType Leaf)) {
            $missingFiles += $relativePath
        }
    }

    $tauriConfigs = @('tauri.conf.json', 'tauri.conf.json5', 'Tauri.toml') |
        ForEach-Object { Join-Path $ProjectRoot ("src-tauri\{0}" -f $_) }
    if (-not ($tauriConfigs | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf })) {
        $missingFiles += 'src-tauri\tauri.conf.json'
    }

    if ($missingFiles.Count -gt 0) {
        throw ("This is an incomplete Agentic Council folder. Missing: {0}. Download or extract the complete project, then run install.bat again." -f ($missingFiles -join ', '))
    }

    $toolchainText = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'rust-toolchain.toml')
    $toolchainMatch = [regex]::Match($toolchainText, '(?m)^\s*channel\s*=\s*"([^"]+)"')
    if ($toolchainMatch.Success) {
        $script:RustToolchain = $toolchainMatch.Groups[1].Value
    }
}

try {
    New-Item -ItemType Directory -Force -Path $script:TempRoot | Out-Null
    Set-Location -LiteralPath $ProjectRoot

    Write-Host ''
    Write-Host '========================================================' -ForegroundColor Cyan
    Write-Host '       Agentic Council - One-Click Windows Setup' -ForegroundColor Cyan
    Write-Host ' Everything is checked, installed, and launched for you.'
    Write-Host '========================================================' -ForegroundColor Cyan
    Write-Info "App folder: $ProjectRoot"

    Write-Step 'Checking that the app package is complete...'
    Assert-CompleteProject
    Write-Ok 'All required application files are present.'
    if ($ValidateOnly) {
        Write-Ok 'Installer validation completed.'
        exit 0
    }

    $architecture = Get-NativeArchitecture

    Write-Step 'Checking Node.js...'
    $node = Find-CompatibleNode
    if ($null -eq $node) {
        $node = Install-PortableNode -NodeArchitecture $architecture.Node
    }
    $nodeHome = Split-Path -Parent $node.Path
    $env:Path = "$nodeHome;$env:Path"
    Write-Ok ("Node.js {0} is ready." -f $node.Version)

    Write-Step 'Checking Rust...'
    $cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    $env:Path = "$cargoBin;$env:Path"
    $rustup = Get-Command rustup.exe -ErrorAction SilentlyContinue
    if ($null -eq $rustup) {
        Install-Rustup -RustTarget $architecture.Rust
        $rustup = Get-Command rustup.exe -ErrorAction SilentlyContinue
    }
    if ($null -eq $rustup) {
        throw 'Rust was installed but rustup.exe was not found.'
    }
    Write-Ok 'Rust toolchain manager is ready.'

    Write-Step 'Checking Microsoft C++ Build Tools...'
    $vcTools = Get-VCToolsInstallation
    if ($null -eq $vcTools) {
        Install-VCTools
        $vcTools = Get-VCToolsInstallation
    }
    Write-Ok "C++ Build Tools are ready at $vcTools"

    Write-Step 'Checking Microsoft WebView2...'
    $webViewVersion = Get-WebView2Version
    if ($null -eq $webViewVersion) {
        $webViewVersion = Install-WebView2
    }
    Write-Ok "WebView2 $webViewVersion is ready."

    Write-Step "Installing Rust $RustToolchain and app dependencies..."
    & $rustup.Source toolchain install $RustToolchain --profile minimal --component rustfmt --component clippy
    if ($LASTEXITCODE -ne 0) {
        throw "Could not install Rust $RustToolchain."
    }

    $packageLock = Join-Path $ProjectRoot 'package-lock.json'
    $nodeModules = Join-Path $ProjectRoot 'node_modules'
    if ((Test-Path -LiteralPath $packageLock -PathType Leaf) -and -not (Test-Path -LiteralPath $nodeModules -PathType Container)) {
        & npm.cmd ci --no-audit --no-fund
    } else {
        & npm.cmd install --no-audit --no-fund
    }
    if ($LASTEXITCODE -ne 0) {
        throw 'Could not install the application dependencies.'
    }
    Write-Ok 'All application dependencies are ready.'

    if ($SkipLaunch) {
        Write-Ok 'Setup completed. Launch was skipped as requested.'
        exit 0
    }

    Write-Host ''
    Write-Host '========================================================' -ForegroundColor Green
    Write-Host ' Setup complete. Agentic Council is starting now.' -ForegroundColor Green
    Write-Host ' The first launch compiles the app and can take a while.'
    Write-Host ' Keep this window open while the app is running.'
    Write-Host '========================================================' -ForegroundColor Green
    Write-Host ''
    & npm.cmd run tauri -- dev
    if ($LASTEXITCODE -ne 0) {
        throw ("Agentic Council exited with code {0}." -f $LASTEXITCODE)
    }
} catch {
    Write-Host ''
    Write-Host 'SETUP COULD NOT FINISH' -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    Write-Host ''
    Write-Host 'Nothing needs to be undone. Fix the message above, then run install.bat again.'
    exit 1
} finally {
    if (Test-Path -LiteralPath $script:TempRoot) {
        $resolvedTemp = [IO.Path]::GetFullPath($script:TempRoot)
        $allowedTemp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
        if ($resolvedTemp.StartsWith($allowedTemp, [StringComparison]::OrdinalIgnoreCase) -and
            (Split-Path -Leaf $resolvedTemp) -like 'AgenticCouncilInstaller-*') {
            Remove-Item -LiteralPath $resolvedTemp -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
