export type UserSession =
    | { mode: "microsoft"; uuid: string; username: string; skinUrl: string | null; capeUrl: string | null }
    | { mode: "offline"; uuid: string; username: string }
    | { mode: "demo"; uuid: string; username: string };
