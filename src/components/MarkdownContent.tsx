import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

export function MarkdownContent({ content, className = "" }: { content: string; className?: string }) {
  return (
    <div className={`markdown-content ${className}`.trim()}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        skipHtml
        components={{
          table: ({ children, node: _node, ...props }) => (
            <div className="markdown-table-wrap">
              <table {...props}>{children}</table>
            </div>
          ),
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
