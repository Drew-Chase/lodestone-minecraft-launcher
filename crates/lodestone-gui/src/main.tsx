import React, {useEffect} from "react";
import {BrowserRouter, Route, Routes, useNavigate} from "react-router-dom";
import ReactDOM from "react-dom/client";
import {HeroUIProvider, ToastProvider} from "@heroui/react";

import "./css/index.css";
import AppShell from "./components/shell/AppShell";
import Home from "./pages/Home";
import InstanceDetail from "./pages/InstanceDetail";
import Login from "./pages/Login";
import ComingSoon from "./pages/ComingSoon";
import Worlds from "./pages/Worlds";
import Settings from "./pages/Settings";
import Discover from "./pages/Discover";
import ContentDetail from "./pages/ContentDetail";
import ErrorBoundary from "./components/shell/ErrorBoundary";
import RequireAuth from "./components/shell/RequireAuth";
import {AuthProvider} from "./context/AuthContext";

// Ensure dark mode class is present so Tailwind/HeroUI dark theme applies.
document.documentElement.classList.add("dark");

const rootEl = document.getElementById("root")!;
ReactDOM.createRoot(rootEl).render(
    <React.StrictMode>
        <BrowserRouter>
            <MainContentRenderer/>
        </BrowserRouter>
    </React.StrictMode>
);

export function MainContentRenderer() {
    const navigate = useNavigate();

    // Suppress browser context menu globally (matches previous behavior in a native desktop shell).
    useEffect(() => {
        const handler = (e: MouseEvent) => e.preventDefault();
        window.addEventListener("contextmenu", handler);
        return () => window.removeEventListener("contextmenu", handler);
    }, []);

    return (
        <HeroUIProvider navigate={navigate}>
            <ToastProvider
                placement="bottom-right"
                toastProps={{
                    shouldShowTimeoutProgress: true,
                    timeout: 3000,
                    variant: "flat",
                }}
            />

            <AuthProvider>
                <Routes>
                    <Route path="/" element={<Login/>}/>
                    <Route element={<RequireAuth/>}>
                        <Route element={<AppShell/>}>
                            <Route path="/library" element={<ErrorBoundary><Home/></ErrorBoundary>}/>
                            <Route path="/library/:slug"
                                   element={<ErrorBoundary><InstanceDetail/></ErrorBoundary>}/>
                            <Route path="/discover" element={<ErrorBoundary><Discover/></ErrorBoundary>}>
                                <Route path=":platform/:id" element={<ContentDetail/>}/>
                            </Route>
                            <Route path="/worlds" element={<ErrorBoundary><Worlds/></ErrorBoundary>}/>
                            <Route path="/servers"
                                   element={<ErrorBoundary><ComingSoon name="Servers"/></ErrorBoundary>}/>
                            <Route path="/friends"
                                   element={<ErrorBoundary><ComingSoon name="Friends"/></ErrorBoundary>}/>
                            <Route path="/downloads"
                                   element={<ErrorBoundary><ComingSoon name="Downloads"/></ErrorBoundary>}/>
                            <Route path="/settings" element={<ErrorBoundary><Settings/></ErrorBoundary>}/>
                        </Route>
                    </Route>
                </Routes>
            </AuthProvider>
        </HeroUIProvider>
    );
}
