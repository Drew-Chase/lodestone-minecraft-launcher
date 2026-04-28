import {useCallback, useEffect, useState} from "react";
import {Button, Input} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import TitleBar from "../components/shell/TitleBar";
import {I} from "../components/shell/icons";
import HeroBanner from "../components/library/HeroBanner";
import QuickActions, {type QuickActionKey} from "../components/library/QuickActions";
import InstanceList from "../components/library/InstanceList";
import {type Instance, configToInstance} from "../components/library/instances";
import NewInstanceModal from "../components/modals/NewInstanceModal";
import ImportModal from "../components/modals/ImportModal";
import JoinServerModal from "../components/modals/JoinServerModal";
import CoopSyncModal from "../components/modals/CoopSyncModal";
import DeleteInstanceModal from "../components/modals/DeleteInstanceModal";

interface InstanceConfig {
    id: number;
    name: string;
    minecraft_version: string;
    loader: string;
    loader_version: string | null;
    java_version: string | null;
    created_at: string;
    last_played: string | null;
    instance_path: string;
    mod_count?: number;
}

export default function Home() {
    const [openModal, setOpenModal] = useState<QuickActionKey | null>(null);
    const closeModal = () => setOpenModal(null);
    const [deleteTarget, setDeleteTarget] = useState<Instance | null>(null);
    const [instances, setInstances] = useState<Instance[]>([]);
    const fetchInstances = useCallback(async () => {
        try {
            const configs = await invoke<InstanceConfig[]>("list_instances");
            setInstances(configs.map(configToInstance));
        } catch {
            setInstances([]);
        }
    }, []);

    useEffect(() => {
        fetchInstances();
        // Auto-refresh when instances are created, deleted, or imported
        const unlisten = listen("instances-changed", () => {
            fetchInstances();
        });
        return () => {
            unlisten.then((fn) => fn());
        };
    }, [fetchInstances]);

    const featured = instances[0];

    return (
        <div className="flex flex-col flex-1 min-w-0 min-h-0 bg-bg-0">
            <TitleBar
                title="Library"
                subtitle={`${instances.length} instance${instances.length !== 1 ? "s" : ""}`}
            >
                <Input
                    placeholder="Search instances…"
                    size="sm"
                    classNames={{
                        base: "w-[240px]",
                        inputWrapper: "bg-[rgba(255,255,255,0.04)] border border-line",
                    }}
                    startContent={<I.search size={14}/>}
                />
                <Button isIconOnly variant="bordered" size="sm" aria-label="Refresh" onPress={fetchInstances}>
                    <I.refresh size={16}/>
                </Button>
                <Button isIconOnly variant="bordered" size="sm" aria-label="Notifications">
                    <I.bell size={16}/>
                </Button>
                <Button
                    color="success"
                    size="sm"
                    className="font-bold"
                    startContent={<I.plus size={14}/>}
                    onPress={() => setOpenModal("new")}
                >
                    New Instance
                </Button>
            </TitleBar>

            <div className="flex-1 overflow-y-auto px-7 pt-6 pb-10">
                {featured && <HeroBanner featured={featured} onDeleteRequest={setDeleteTarget}/>}
                <QuickActions onActionPress={setOpenModal}/>
                <InstanceList instances={instances} onDeleteRequest={setDeleteTarget}/>
            </div>

            {/* Quick-action modals */}
            <NewInstanceModal isOpen={openModal === "new"} onClose={closeModal} onCreated={fetchInstances}/>
            <ImportModal isOpen={openModal === "import"} onClose={closeModal}/>
            <JoinServerModal isOpen={openModal === "server"} onClose={closeModal}/>
            <CoopSyncModal isOpen={openModal === "coop"} onClose={closeModal}/>
            <DeleteInstanceModal
                isOpen={deleteTarget !== null}
                instance={deleteTarget}
                onClose={() => setDeleteTarget(null)}
                onDeleted={fetchInstances}
            />
        </div>
    );
}
