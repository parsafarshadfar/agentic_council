use crate::models::{
    CredentialStatus, ModelInfo, Persona, ProviderSummary, TimeoutPolicy, WireProtocol,
};

pub fn providers() -> Vec<ProviderSummary> {
    let rows = [
        (
            "openrouter",
            "OpenRouter",
            WireProtocol::OpenAi,
            "https://openrouter.ai/api/v1",
            true,
            false,
        ),
        (
            "demo",
            "Local Demo",
            WireProtocol::Demo,
            "local://demo",
            false,
            false,
        ),
        (
            "openai",
            "OpenAI",
            WireProtocol::OpenAi,
            "https://api.openai.com/v1",
            true,
            true,
        ),
        (
            "anthropic",
            "Anthropic",
            WireProtocol::Anthropic,
            "https://api.anthropic.com/v1",
            true,
            false,
        ),
        (
            "gemini",
            "Google Gemini",
            WireProtocol::Gemini,
            "https://generativelanguage.googleapis.com/v1beta",
            true,
            false,
        ),
        (
            "deepseek",
            "DeepSeek",
            WireProtocol::OpenAi,
            "https://api.deepseek.com",
            false,
            false,
        ),
        (
            "xai",
            "xAI",
            WireProtocol::OpenAi,
            "https://api.x.ai/v1",
            true,
            false,
        ),
        (
            "groq",
            "Groq",
            WireProtocol::OpenAi,
            "https://api.groq.com/openai/v1",
            true,
            false,
        ),
        (
            "together",
            "Together AI",
            WireProtocol::OpenAi,
            "https://api.together.xyz/v1",
            true,
            false,
        ),
        (
            "fireworks",
            "Fireworks AI",
            WireProtocol::OpenAi,
            "https://api.fireworks.ai/inference/v1",
            true,
            false,
        ),
        (
            "siliconflow",
            "SiliconFlow",
            WireProtocol::OpenAi,
            "https://api.siliconflow.com/v1",
            true,
            false,
        ),
        (
            "huggingface",
            "Hugging Face",
            WireProtocol::OpenAi,
            "https://router.huggingface.co/v1",
            true,
            false,
        ),
        (
            "perplexity",
            "Perplexity",
            WireProtocol::OpenAi,
            "https://api.perplexity.ai",
            false,
            false,
        ),
        (
            "dashscope",
            "Alibaba DashScope",
            WireProtocol::OpenAi,
            "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
            false,
            false,
        ),
        (
            "moonshot",
            "Moonshot AI",
            WireProtocol::OpenAi,
            "https://api.moonshot.ai/v1",
            false,
            false,
        ),
        (
            "zai",
            "Z.AI",
            WireProtocol::OpenAi,
            "https://api.z.ai/api/paas/v4",
            false,
            false,
        ),
        (
            "inference_net",
            "Inference.net",
            WireProtocol::OpenAi,
            "https://api.inference.net/v1",
            false,
            false,
        ),
        (
            "custom",
            "Custom OpenAI-compatible",
            WireProtocol::OpenAi,
            "https://example.invalid/v1",
            false,
            true,
        ),
    ];
    rows.into_iter()
        .map(|(id, name, protocol, base_url, discovery, configurable)| {
            let mut timeout = TimeoutPolicy::default();
            if id == "deepseek" {
                timeout.first_token_secs = 113;
            }
            ProviderSummary {
                id: id.to_string(),
                name: name.to_string(),
                base_url: base_url.to_string(),
                protocol,
                credential_status: if id == "demo" {
                    CredentialStatus::NotRequired
                } else {
                    CredentialStatus::Untested
                },
                supports_discovery: discovery,
                configurable_endpoint: configurable,
                timeout,
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn model(
    id: &str,
    provider: &str,
    name: &str,
    context: u64,
    input: Option<f64>,
    output: Option<f64>,
    vision: bool,
    reasoning: bool,
) -> ModelInfo {
    ModelInfo {
        id: id.into(),
        provider_id: provider.into(),
        name: name.into(),
        context_window: context,
        input_per_million: input,
        output_per_million: output,
        supports_vision: vision,
        supports_documents: true,
        supports_streaming: true,
        reasoning,
    }
}

pub fn models() -> Vec<ModelInfo> {
    vec![
        model(
            "council-demo",
            "demo",
            "Council Demo (offline)",
            64_000,
            Some(0.0),
            Some(0.0),
            true,
            false,
        ),
        model(
            "gpt-4.1",
            "openai",
            "GPT-4.1",
            1_047_576,
            Some(2.0),
            Some(8.0),
            true,
            false,
        ),
        model(
            "o3",
            "openai",
            "o3",
            200_000,
            Some(2.0),
            Some(8.0),
            true,
            true,
        ),
        model(
            "claude-sonnet-4",
            "anthropic",
            "Claude Sonnet 4",
            200_000,
            Some(3.0),
            Some(15.0),
            true,
            true,
        ),
        model(
            "gemini-2.5-pro",
            "gemini",
            "Gemini 2.5 Pro",
            1_048_576,
            Some(1.25),
            Some(10.0),
            true,
            true,
        ),
        model(
            "deepseek-v4-flash",
            "deepseek",
            "DeepSeek V4 Flash",
            128_000,
            None,
            None,
            false,
            true,
        ),
        model(
            "deepseek-v4-pro",
            "deepseek",
            "DeepSeek V4 Pro",
            128_000,
            None,
            None,
            false,
            true,
        ),
        model(
            "grok-3",
            "xai",
            "Grok 3",
            131_072,
            Some(3.0),
            Some(15.0),
            false,
            true,
        ),
        model(
            "openai/gpt-4.1",
            "openrouter",
            "GPT-4.1 via OpenRouter",
            1_047_576,
            Some(2.0),
            Some(8.0),
            true,
            false,
        ),
        model(
            "llama-3.3-70b-versatile",
            "groq",
            "Llama 3.3 70B",
            128_000,
            Some(0.59),
            Some(0.79),
            false,
            false,
        ),
        model(
            "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            "together",
            "Llama 3.3 70B Turbo",
            131_072,
            Some(0.88),
            Some(0.88),
            false,
            false,
        ),
        model(
            "accounts/fireworks/models/llama-v3p3-70b-instruct",
            "fireworks",
            "Llama 3.3 70B",
            131_072,
            Some(0.9),
            Some(0.9),
            false,
            false,
        ),
        model(
            "Qwen/Qwen2.5-72B-Instruct",
            "siliconflow",
            "Qwen 2.5 72B",
            32_768,
            Some(0.56),
            Some(0.56),
            false,
            false,
        ),
        model(
            "openai/gpt-oss-120b:fastest",
            "huggingface",
            "GPT OSS 120B (fastest route)",
            128_000,
            None,
            None,
            false,
            false,
        ),
        model(
            "sonar-pro",
            "perplexity",
            "Sonar Pro",
            200_000,
            Some(3.0),
            Some(15.0),
            false,
            false,
        ),
        model(
            "qwen-plus",
            "dashscope",
            "Qwen Plus",
            131_072,
            None,
            None,
            false,
            false,
        ),
        model(
            "moonshot-v1-128k",
            "moonshot",
            "Moonshot 128K",
            128_000,
            None,
            None,
            false,
            false,
        ),
        model(
            "glm-5.1", "zai", "GLM 5.1", 202_752, None, None, false, true,
        ),
        model(
            "meta-llama/llama-3.3-70b-instruct",
            "inference_net",
            "Llama 3.3 70B",
            128_000,
            None,
            None,
            false,
            false,
        ),
        model(
            "custom-model",
            "custom",
            "Custom model",
            32_768,
            None,
            None,
            false,
            false,
        ),
    ]
}

fn persona(id: &str, name: &str, description: &str, prompt: &str, directives: &[&str]) -> Persona {
    Persona {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        system_prompt: prompt.into(),
        directives: directives
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        builtin: true,
    }
}

pub fn personas() -> Vec<Persona> {
    vec![
        persona(
            "devils-advocate",
            "Devil's Advocate",
            "Challenges assumptions and exposes omissions.",
            "Question assumptions, probe logic, expose omissions, and challenge premature consensus.",
            &[
                "Name the strongest counterargument",
                "Separate evidence from assertion",
            ],
        ),
        persona(
            "visionary",
            "Visionary Product Innovator",
            "Optimizes for a delightful, disruptive product.",
            "Prioritize user experience, simplicity, aesthetic detail, and disruptive product thinking.",
            &[
                "Start from the user outcome",
                "Look for a 10x simplification",
            ],
        ),
        persona(
            "first-principles",
            "First-Principles Simplifier",
            "Reduces the problem to fundamental truths.",
            "Decompose the problem into fundamentals, remove jargon, and identify unnecessary assumptions.",
            &[
                "Define irreducible constraints",
                "Prefer the simplest sufficient model",
            ],
        ),
        persona(
            "strategist",
            "Pragmatic Strategist",
            "Tests viability, incentives, and competitive position.",
            "Analyze incentives, power dynamics, competitive advantage, hidden risk, and pragmatic viability.",
            &[
                "Identify the binding constraint",
                "Make trade-offs explicit",
            ],
        ),
        persona(
            "architect",
            "Technical Architect",
            "Evaluates design quality and operational behavior.",
            "Evaluate architecture, complexity, performance, reliability, security, and maintainability.",
            &["Quantify scale assumptions", "Trace failure modes"],
        ),
        persona(
            "ethical-guardian",
            "Ethical Guardian",
            "Assesses consequences, fairness, and resilience.",
            "Assess long-term consequences, stakeholder impact, ethical risk, balance, and resilience.",
            &[
                "Include affected non-users",
                "Test misuse and reversibility",
            ],
        ),
    ]
}
