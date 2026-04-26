import {createContext, useContext, useEffect, useState, useCallback, type ReactNode} from "react";
import {invoke} from "@tauri-apps/api/core";
import type {UserSession} from "../types/auth";

interface AuthContextValue {
    session: UserSession | null;
    loading: boolean;
    error: string | null;
    loginMicrosoft: () => Promise<void>;
    loginOffline: (username: string) => Promise<void>;
    loginDemo: () => Promise<void>;
    logout: () => Promise<void>;
    clearError: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function useAuth(): AuthContextValue {
    const ctx = useContext(AuthContext);
    if (!ctx) throw new Error("useAuth must be used within <AuthProvider>");
    return ctx;
}

export function AuthProvider({children}: { children: ReactNode }) {
    const [session, setSession] = useState<UserSession | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    // Restore session on mount.
    useEffect(() => {
        invoke<UserSession | null>("restore_session")
            .then((s) => setSession(s))
            .catch((e) => console.warn("session restore failed:", e))
            .finally(() => setLoading(false));
    }, []);

    const loginMicrosoft = useCallback(async () => {
        setError(null);
        try {
            const s = await invoke<UserSession>("login_microsoft");
            setSession(s);
        } catch (e) {
            setError(typeof e === "string" ? e : String(e));
            throw e;
        }
    }, []);

    const loginOffline = useCallback(async (username: string) => {
        setError(null);
        try {
            const s = await invoke<UserSession>("login_offline", {username});
            setSession(s);
        } catch (e) {
            setError(typeof e === "string" ? e : String(e));
            throw e;
        }
    }, []);

    const loginDemo = useCallback(async () => {
        setError(null);
        try {
            const s = await invoke<UserSession>("login_demo");
            setSession(s);
        } catch (e) {
            setError(typeof e === "string" ? e : String(e));
            throw e;
        }
    }, []);

    const logout = useCallback(async () => {
        try {
            await invoke("logout");
        } catch (e) {
            console.warn("logout error:", e);
        }
        setSession(null);
        setError(null);
    }, []);

    const clearError = useCallback(() => setError(null), []);

    return (
        <AuthContext.Provider
            value={{session, loading, error, loginMicrosoft, loginOffline, loginDemo, logout, clearError}}
        >
            {children}
        </AuthContext.Provider>
    );
}
