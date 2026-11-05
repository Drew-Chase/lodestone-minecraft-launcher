import {useContext} from "react";
import {AuthContext} from "./context.ts";
import type {AuthContextValue} from "./context.ts";

export function useAuth(): AuthContextValue {
    const value = useContext(AuthContext);
    if (!value) {
        throw new Error("useAuth must be used inside AuthProvider");
    }
    return value;
}
