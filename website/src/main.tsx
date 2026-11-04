import React from "react";
import ReactDOM from "react-dom/client";

import "./css/index.css";
import Home from "./pages/Home.tsx";
import Navigation from "./components/Navigation.tsx";
import {useRelease, ReleaseContext} from "./hooks/useRelease.ts";

function App() {
    const release = useRelease();

    return (
        <ReleaseContext.Provider value={release}>
            <Navigation/>
            <Home/>
        </ReleaseContext.Provider>
    );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
        <App/>
    </React.StrictMode>
);
