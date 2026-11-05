// Shapes from api/README.md. Keep these in step with the API.

export interface User {
    id: number;
    email: string;
    email_verified: boolean;
    display_name: string | null;
    presence_hidden: boolean;
    created_at: string;
}

export interface Session {
    access_token: string;
    token_type: "Bearer";
    expires_in: number;
    user: User;
}

export interface StatusResponse {
    status: string;
}
