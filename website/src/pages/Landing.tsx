import Navigation from "../components/Navigation.tsx";
import ScrollSpy from "../components/ScrollSpy.tsx";
import {ReleaseContext, useRelease} from "../hooks/useRelease.ts";
import Home from "./Home.tsx";

// The one-page site at `/`. It is the same page as before routing was added.
export default function Landing() {
    const release = useRelease();

    return (
        <ReleaseContext.Provider value={release}>
            <Navigation/>
            <ScrollSpy/>
            <Home/>
        </ReleaseContext.Provider>
    );
}
