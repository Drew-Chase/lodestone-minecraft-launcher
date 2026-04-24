import {useState} from "react";
import {Button, Input} from "@heroui/react";
import TitleBar from "../components/shell/TitleBar";
import {I} from "../components/shell/icons";
import HeroBanner from "../components/library/HeroBanner";
import QuickActions, {type QuickActionKey} from "../components/library/QuickActions";
import InstanceList from "../components/library/InstanceList";
import {instances} from "../components/library/instances";
import NewInstanceModal from "../components/modals/NewInstanceModal";
import ImportModal from "../components/modals/ImportModal";
import JoinServerModal from "../components/modals/JoinServerModal";
import CoopSyncModal from "../components/modals/CoopSyncModal";

export default function Home() {
    const [openModal, setOpenModal] = useState<QuickActionKey | null>(null);
    const closeModal = () => setOpenModal(null);

    const featured = instances[0];

    return (
        <div className="flex flex-col flex-1 min-w-0 min-h-0 bg-bg-0">
            <TitleBar title="Library" subtitle="6 instances · 300h total playtime">
                <Input
                    placeholder="Search instances…"
                    size="sm"
                    classNames={{
                        base: "w-[240px]",
                        inputWrapper: "bg-[rgba(255,255,255,0.04)] border border-line",
                    }}
                    startContent={<I.search size={14}/>}
                />
                <Button isIconOnly variant="bordered" size="sm" aria-label="Refresh">
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
                <HeroBanner featured={featured}/>
                <QuickActions onActionPress={setOpenModal}/>
                <InstanceList instances={instances}/>
            </div>

            {/* Quick-action modals — mounted here so they can overlay the whole Library page. */}
            <NewInstanceModal isOpen={openModal === "new"} onClose={closeModal}/>
            <ImportModal isOpen={openModal === "import"} onClose={closeModal}/>
            <JoinServerModal isOpen={openModal === "server"} onClose={closeModal}/>
            <CoopSyncModal isOpen={openModal === "coop"} onClose={closeModal}/>
        </div>
    );
}
