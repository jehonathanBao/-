import { useEffect, useState } from "react";
import { fetchUsageGuide } from "../api/usageGuide.js";

export default function UsageGuide() {
  const [guide, setGuide] = useState({
    error: null,
    loading: true,
    markdown: "",
    sourcePath: "docs/usage-guide.md",
    title: "有毒订单监控用户使用指南",
  });

  useEffect(() => {
    let cancelled = false;
    fetchUsageGuide()
      .then((payload) => {
        if (!cancelled) {
          setGuide({ ...payload, error: null, loading: false });
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setGuide((current) => ({
            ...current,
            error: error?.message || "usage_guide_unavailable",
            loading: false,
          }));
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const blocks = parseMarkdown(guide.markdown);

  return (
    <article className="rounded-2xl border border-slate-700/60 bg-slate-900/70 shadow-glow">
      <div className="border-b border-slate-700/60 px-5 py-5">
        <p className="text-xs uppercase tracking-[0.3em] text-cyan-300">Operator Guide</p>
        <h2 className="mt-2 text-2xl font-bold text-white">使用指南</h2>
        <p className="mt-2 max-w-3xl text-sm leading-6 text-slate-400">
          {guide.sourcePath} · 信号解读、页面状态、Discord 提示和日常看盘顺序。
        </p>
      </div>
      <div className="usage-guide-content px-5 py-6">
        {guide.loading ? (
          <p className="text-sm text-slate-300">使用指南加载中...</p>
        ) : guide.error ? (
          <p className="rounded-xl border border-red-400/40 bg-red-500/10 px-4 py-3 text-sm text-red-200">
            使用指南加载失败：{guide.error}
          </p>
        ) : (
          blocks.map((block, index) => renderBlock(block, index))
        )}
      </div>
    </article>
  );
}

function parseMarkdown(markdown) {
  const lines = markdown.replace(/\r\n/g, "\n").split("\n");
  const blocks = [];
  let paragraph = [];
  let list = null;
  let code = null;

  function flushParagraph() {
    if (paragraph.length > 0) {
      blocks.push({ type: "paragraph", text: paragraph.join(" ") });
      paragraph = [];
    }
  }

  function flushList() {
    if (list) {
      blocks.push(list);
      list = null;
    }
  }

  for (const line of lines) {
    if (code) {
      if (line.startsWith("```")) {
        blocks.push(code);
        code = null;
      } else {
        code.lines.push(line);
      }
      continue;
    }

    if (line.startsWith("```")) {
      flushParagraph();
      flushList();
      code = { type: "code", language: line.slice(3).trim(), lines: [] };
      continue;
    }

    const heading = /^(#{1,3})\s+(.*)$/.exec(line);
    if (heading) {
      flushParagraph();
      flushList();
      blocks.push({ type: "heading", level: heading[1].length, text: heading[2] });
      continue;
    }

    const unordered = /^-\s+(.*)$/.exec(line);
    if (unordered) {
      flushParagraph();
      if (!list || list.type !== "unordered") {
        flushList();
        list = { type: "unordered", items: [] };
      }
      list.items.push(unordered[1]);
      continue;
    }

    const ordered = /^\d+\.\s+(.*)$/.exec(line);
    if (ordered) {
      flushParagraph();
      if (!list || list.type !== "ordered") {
        flushList();
        list = { type: "ordered", items: [] };
      }
      list.items.push(ordered[1]);
      continue;
    }

    if (!line.trim()) {
      flushParagraph();
      flushList();
      continue;
    }

    flushList();
    paragraph.push(line.trim());
  }

  flushParagraph();
  flushList();
  if (code) {
    blocks.push(code);
  }

  return blocks;
}

function renderBlock(block, index) {
  if (block.type === "heading") {
    if (block.level === 1) {
      return (
        <h1 className="mb-5 text-3xl font-black text-white" key={index}>
          {renderInline(block.text)}
        </h1>
      );
    }
    if (block.level === 2) {
      return (
        <h3 className="mb-3 mt-8 border-l-2 border-cyan-300 pl-3 text-xl font-bold text-white" key={index}>
          {renderInline(block.text)}
        </h3>
      );
    }
    return (
      <h4 className="mb-2 mt-5 text-base font-bold text-cyan-100" key={index}>
        {renderInline(block.text)}
      </h4>
    );
  }

  if (block.type === "paragraph") {
    return (
      <p className="mb-4 max-w-4xl text-sm leading-7 text-slate-300" key={index}>
        {renderInline(block.text)}
      </p>
    );
  }

  if (block.type === "unordered") {
    return (
      <ul className="mb-5 ml-5 list-disc space-y-2 text-sm leading-7 text-slate-300" key={index}>
        {block.items.map((item, itemIndex) => (
          <li key={itemIndex}>{renderInline(item)}</li>
        ))}
      </ul>
    );
  }

  if (block.type === "ordered") {
    return (
      <ol className="mb-5 ml-5 list-decimal space-y-2 text-sm leading-7 text-slate-300" key={index}>
        {block.items.map((item, itemIndex) => (
          <li key={itemIndex}>{renderInline(item)}</li>
        ))}
      </ol>
    );
  }

  if (block.type === "code") {
    return (
      <pre className="mb-5 overflow-x-auto rounded-xl border border-slate-700/70 bg-slate-950/90 p-4 text-xs leading-6 text-cyan-100" key={index}>
        <code>{block.lines.join("\n")}</code>
      </pre>
    );
  }

  return null;
}

function renderInline(text) {
  const parts = String(text).split(/(`[^`]+`)/g);
  return parts.map((part, index) => {
    if (part.startsWith("`") && part.endsWith("`")) {
      return (
        <code className="rounded-md border border-cyan-400/20 bg-cyan-400/10 px-1.5 py-0.5 text-cyan-100" key={index}>
          {part.slice(1, -1)}
        </code>
      );
    }
    return <span key={index}>{part}</span>;
  });
}
