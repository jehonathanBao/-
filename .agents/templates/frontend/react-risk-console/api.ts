import axios from "axios";

export const riskApi = axios.create({
  baseURL: import.meta.env.VITE_RISK_API_BASE_URL ?? "/api",
  timeout: 10_000,
});

riskApi.interceptors.request.use((config) => {
  const token = window.sessionStorage.getItem("operator_api_token");
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});
