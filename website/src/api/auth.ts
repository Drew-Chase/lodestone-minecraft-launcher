// The account endpoints, one function each. They return the API's answer and throw `ApiError` on failure. The auth
// provider decides what the answer means for the session.
import {apiRequest} from "./client.ts";
import type {Session, StatusResponse} from "./types.ts";

export function login(email: string, password: string): Promise<Session> {
    return apiRequest<Session>("/api/auth/login", {method: "POST", body: {email, password}});
}

export function register(email: string, password: string, displayName?: string): Promise<StatusResponse> {
    const body = displayName ? {email, password, display_name: displayName} : {email, password};
    return apiRequest<StatusResponse>("/api/auth/register", {method: "POST", body});
}

export function resendVerification(email: string): Promise<StatusResponse> {
    return apiRequest<StatusResponse>("/api/auth/resend-verification", {method: "POST", body: {email}});
}

export function verifyEmail(token: string): Promise<StatusResponse> {
    return apiRequest<StatusResponse>("/api/auth/verify-email", {method: "POST", body: {token}});
}

export function requestPasswordReset(email: string): Promise<StatusResponse> {
    return apiRequest<StatusResponse>("/api/auth/forgot-password", {method: "POST", body: {email}});
}

export function resetPassword(token: string, password: string): Promise<StatusResponse> {
    return apiRequest<StatusResponse>("/api/auth/reset-password", {method: "POST", body: {token, password}});
}

export function changePassword(currentPassword: string, newPassword: string): Promise<StatusResponse> {
    return apiRequest<StatusResponse>("/api/auth/change-password", {
        method: "POST",
        body: {current_password: currentPassword, new_password: newPassword},
        auth: true,
    });
}
