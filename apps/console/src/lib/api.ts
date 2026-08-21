/**
 * Shifa API Client for Console (Doc 16)
 * Typed, token-authenticated HTTP client connecting to /api/v1.
 */

const API_BASE = typeof window !== "undefined"
  ? (window as any).__SHIFA_API_BASE__ || ""
  : "";

export interface ApiFetchOptions extends RequestInit {
  token?: string;
  tenantId?: string;
}

export async function apiFetch<T>(path: string, options: ApiFetchOptions = {}): Promise<T> {
  const token = options.token || (typeof localStorage !== "undefined" ? localStorage.getItem("shifa_auth_token") : null);
  const tenantId = options.tenantId || (typeof localStorage !== "undefined" ? localStorage.getItem("shifa_tenant_id") : null);

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(options.headers as Record<string, string>),
  };

  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }
  if (tenantId) {
    headers["X-Tenant-ID"] = tenantId;
  }

  const url = `${API_BASE}${path}`;
  const response = await fetch(url, {
    ...options,
    headers,
  });

  if (!response.ok) {
    let errorMsg = `API Error ${response.status}: ${response.statusText}`;
    try {
      const errorJson = await response.json();
      if (errorJson?.error?.message) {
        errorMsg = errorJson.error.message;
      }
    } catch {
      // Ignore parse failure
    }
    throw new Error(errorMsg);
  }

  const contentType = response.headers.get("content-type");
  if (contentType && contentType.includes("text/csv")) {
    return (await response.text()) as unknown as T;
  }

  return await response.json();
}
