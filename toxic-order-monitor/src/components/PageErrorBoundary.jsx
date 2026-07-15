import React from "react";
import PageShellSkeleton from "./PageShellSkeleton.jsx";

export class PageErrorBoundary extends React.Component {
  constructor(props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error) {
    return { hasError: true, error };
  }

  componentDidCatch(error, info) {
    console.error("page_error_boundary_caught", error, info);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      return (
        <div data-testid="page-error-boundary">
          <PageShellSkeleton />
          <div className="workspace-dialog mx-auto -mt-28 max-w-3xl px-4 py-3 text-sm text-yellow-100">
            <p className="font-semibold">页面加载失败</p>
            <p className="mt-1 text-xs leading-5 text-yellow-100/80">
              当前页面壳已保留，你可以重试或稍后刷新。下面的错误只影响这一页，不会让整站白屏。
            </p>
            <button
              className="mt-3 rounded-lg border border-cyan-500/30 px-3 py-1.5 text-xs font-semibold text-cyan-100 transition hover:bg-cyan-500/10"
              onClick={this.handleReset}
              type="button"
            >
              重试
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

export default PageErrorBoundary;
