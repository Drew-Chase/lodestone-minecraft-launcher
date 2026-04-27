import {Dropdown, DropdownTrigger, DropdownMenu, DropdownItem, DropdownSection} from "@heroui/react";
import {useNavigate} from "react-router-dom";
import {invoke} from "@tauri-apps/api/core";
import {I} from "../shell/icons";
import {toSlug, type Instance} from "./instances";

type Props = {
    instance: Instance;
    onDeleteRequest: (inst: Instance) => void;
    children: React.ReactNode;
};

export default function InstanceActionsDropdown({instance, onDeleteRequest, children}: Props) {
    const navigate = useNavigate();

    const handleAction = async (key: React.Key) => {
        switch (key) {
            case "open-dir":
                await invoke("open_directory", {path: instance.instancePath});
                break;
            case "settings":
                navigate(`/library/${toSlug(instance.name)}?tab=settings`);
                break;
            case "delete":
                onDeleteRequest(instance);
                break;
        }
    };

    return (
        <Dropdown
            classNames={{
                content: "bg-[#0d1117] border border-line min-w-[180px]",
            }}
        >
            <DropdownTrigger>{children}</DropdownTrigger>
            <DropdownMenu
                aria-label="Instance actions"
                onAction={handleAction}
                itemClasses={{
                    base: "text-xs text-[var(--ink-2)] data-[hover=true]:bg-white/[0.04] data-[selectable=true]:focus:bg-white/[0.04] gap-2",
                }}
            >
                <DropdownSection showDivider>
                    <DropdownItem key="open-dir" startContent={<I.folder size={14}/>}>
                        Open Directory
                    </DropdownItem>
                    <DropdownItem key="settings" startContent={<I.settings size={14}/>}>
                        Settings
                    </DropdownItem>
                </DropdownSection>
                <DropdownSection showDivider>
                    <DropdownItem key="play-offline" startContent={<I.wifi size={14}/>} isDisabled description="Coming soon">
                        Play Offline
                    </DropdownItem>
                    <DropdownItem key="reinstall" startContent={<I.refresh size={14}/>} isDisabled description="Coming soon">
                        Re-install
                    </DropdownItem>
                    <DropdownItem key="export" startContent={<I.upload size={14}/>} isDisabled description="Coming soon">
                        Export
                    </DropdownItem>
                </DropdownSection>
                <DropdownSection>
                    <DropdownItem key="delete" startContent={<I.trash size={14}/>} className="text-danger" color="danger">
                        Delete
                    </DropdownItem>
                </DropdownSection>
            </DropdownMenu>
        </Dropdown>
    );
}
