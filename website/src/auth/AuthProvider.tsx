import {useCallback, useMemo, useSyncExternalStore} from "react";
import type {ReactNode} from "react";
import * as api from "../api/auth.ts";
import {isSessionEnded, logout, refreshSession} from "../api/client.ts";
import {getSessionState, setSignedIn, setSignedOut, subscribeSession} from "../api/session.ts";
import type {User} from "../api/types.ts";
import {AuthContext} from "./context.ts";
import type {AuthContextValue} from "./context.ts";

// Nothing refreshes on load. A visitor with no session would only get a 401, so the account page asks for one itself.
export function AuthProvider({children}: { children: ReactNode }) {
    const session = useSyncExternalStore(subscribeSession, getSessionState, getSessionState);

    const ensureSession = useCallback(async (): Promise<User | null> => {
        const current = getSessionState();
        if (current.status === "signed_in" && current.user) return current.user;
        try {
            // refreshSession shares one request between concurrent callers, because a refresh token is spent once.
            const fresh = await refreshSession();
            return fresh.user;
        } catch (error) {
            // An ended session has already marked the browser signed out. Any other failure, such as a network error,
            // leaves the state alone and is thrown so the caller can report it.
            if (isSessionEnded(error)) return null;
            throw error;
        }
    }, []);

    const value = useMemo<AuthContextValue>(() => ({
        status: session.status,
        user: session.user,
        ensureSession,

        async signIn(email, password) {
            setSignedIn(await api.login(email, password));
        },
        async register(email, password, displayName) {
            await api.register(email, password, displayName);
        },
        async resendVerification(email) {
            await api.resendVerification(email);
        },
        async verifyEmail(token) {
            await api.verifyEmail(token);
        },
        async requestPasswordReset(email) {
            await api.requestPasswordReset(email);
        },
        async resetPassword(token, password) {
            await api.resetPassword(token, password);
            // The server revokes every session on a reset, so this browser's session ends too.
            setSignedOut();
        },
        async changePassword(currentPassword, newPassword) {
            await api.changePassword(currentPassword, newPassword);
        },
        async signOut() {
            try {
                await logout();
            } catch {
                // The browser is signed out below either way, so a failed logout call is not shown.
            }
            setSignedOut();
        },
    }), [session, ensureSession]);

    return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
