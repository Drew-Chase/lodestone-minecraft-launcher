import {Outlet} from "react-router";
import {AuthProvider} from "./AuthProvider.tsx";

// Wraps the account routes only. The landing, legal and not-found pages sit outside it, so they never touch the session.
export default function AuthLayout() {
    return (
        <AuthProvider>
            <Outlet/>
        </AuthProvider>
    );
}
