import {Navigate, Outlet} from "react-router-dom";
import {useAuth} from "../../context/AuthContext";

export default function RequireAuth() {
    const {session, loading} = useAuth();

    if (loading) {
        return (
            <div className="flex items-center justify-center h-screen w-screen bg-bg-0">
                <div className="flex flex-col items-center gap-3">
                    <div className="w-8 h-8 border-2 border-mc-green border-t-transparent rounded-full animate-spin"/>
                    <span className="text-sm text-ink-3 font-mono">RESTORING SESSION</span>
                </div>
            </div>
        );
    }

    if (!session) {
        return <Navigate to="/" replace/>;
    }

    return <Outlet/>;
}
