import {Outlet, useMatch} from "react-router-dom";
import TitleBar from "../components/shell/TitleBar";
import ContentBrowser from "../components/discover/ContentBrowser";

export default function Discover() {
    const detailMatch = useMatch("/discover/:platform/:id");
    const showDetail = !!detailMatch;

    return (
        <div className="flex-1 flex flex-col overflow-hidden relative" style={{background: "var(--bg-0)"}}>
            <div className="flex-shrink-0">
                <TitleBar title="Discover" subtitle="Browse mods, modpacks, shaders, and more"/>
            </div>

            <ContentBrowser/>

            {/* Content detail overlay — Discover stays mounted underneath */}
            {showDetail && (
                <div className="absolute inset-0 z-30 flex flex-col" style={{background: "var(--bg-0)"}}>
                    <Outlet/>
                </div>
            )}
        </div>
    );
}
