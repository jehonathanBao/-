import axios from "axios";

export async function fetchUsageGuide() {
  const response = await axios.get(`${apiBaseUrl()}/api/docs/usage-guide`);
  const markdown = typeof response.data?.markdown === "string" ? response.data.markdown : "";
  return {
    markdown,
    readOnly: response.data?.readOnly !== false,
    sourcePath: response.data?.sourcePath || "docs/usage-guide.md",
    title: response.data?.title || "有毒订单监控用户使用指南",
  };
}

function apiBaseUrl() {
  return (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
}
