export type ProviderName = "google" | "microsoft" | "github";

// The single sign-on providers that are configured on the server. It never throws: a network error or a non-200
// response gives an empty list, so the sign-in form still works without the buttons.
export async function fetchProviders(): Promise<ProviderName[]> {
    try {
        const response = await fetch("/api/auth/providers", {
            method: "GET",
            headers: {Accept: "application/json"},
            credentials: "include",
        });
        if (response.status !== 200) return [];
        const data: unknown = await response.json();
        if (typeof data !== "object" || data === null || !("providers" in data)) return [];
        const providers = (data as { providers: unknown }).providers;
        return Array.isArray(providers) ? (providers as ProviderName[]) : [];
    } catch {
        return [];
    }
}
