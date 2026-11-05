import {createContext} from "react";
import type {AuthStatus} from "../api/session.ts";
import type {User} from "../api/types.ts";

export interface AuthContextValue {
    status: AuthStatus;
    user: User | null;
    /**
     * Returns the signed-in user, refreshing the session if there is none. A held session is returned without a network
     * call. Resolves null when the session has ended. Rejects with an `ApiError` (for example `network_error`) when the
     * refresh could not be completed; the signed-in state is left as it was.
     */
    ensureSession(): Promise<User | null>;
    signIn(email: string, password: string): Promise<void>;
    register(email: string, password: string, displayName?: string): Promise<void>;
    resendVerification(email: string): Promise<void>;
    verifyEmail(token: string): Promise<void>;
    requestPasswordReset(email: string): Promise<void>;
    resetPassword(token: string, password: string): Promise<void>;
    changePassword(currentPassword: string, newPassword: string): Promise<void>;
    signOut(): Promise<void>;
}

export const AuthContext = createContext<AuthContextValue | null>(null);
