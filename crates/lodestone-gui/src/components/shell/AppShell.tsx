import {Outlet} from "react-router-dom";
import WindowChrome from "./WindowChrome";
import Sidebar from "./Sidebar";

// Layout for all in-app routes: titlebar on top, sidebar on left, page content on right.
export default function AppShell() {
    return (
        <div className="flex flex-col h-screen w-screen overflow-hidden bg-bg-0">
            <WindowChrome/>
            <div className="flex flex-row flex-1 min-h-0">
                <Sidebar/>
                <div className="flex flex-col flex-1 min-w-0 min-h-0">
                    <Outlet/>
                </div>
            </div>
        </div>
    );
}
