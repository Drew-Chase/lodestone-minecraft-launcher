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

            <Routes>
                <Route path="/" element={<Login/>}/>
                <Route element={<AppShell/>}>
                    <Route path="/library" element={<Home/>}/>
                    <Route path="/library/:slug" element={<InstanceDetail/>}/>
                    <Route path="/discover" element={<ComingSoon name="Discover"/>}/>
                    <Route path="/worlds" element={<ComingSoon name="Worlds"/>}/>
                    <Route path="/servers" element={<ComingSoon name="Servers"/>}/>
                    <Route path="/friends" element={<ComingSoon name="Friends"/>}/>
                    <Route path="/downloads" element={<ComingSoon name="Downloads"/>}/>
                    <Route path="/settings" element={<ComingSoon name="Settings"/>}/>
                </Route>
            </Routes>
        </HeroUIProvider>
    );
}
