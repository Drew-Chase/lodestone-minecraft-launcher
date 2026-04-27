import {useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import ModalShell from "./ModalShell";
import {FooterBtn} from "./primitives";
import {I} from "../shell/icons";
import type {Instance} from "../library/instances";

type Props = {
    isOpen: boolean;
    instance: Instance | null;
    onClose: () => void;
    onDeleted: () => void;
};

export default function DeleteInstanceModal({isOpen, instance, onClose, onDeleted}: Props) {
    const [loading, setLoading] = useState(false);

    const handleDelete = async () => {
        if (!instance) return;
        setLoading(true);
        try {
            await invoke("delete_instance", {id: instance.id});
            onDeleted();
            onClose();
        } catch (e) {
            console.error("Failed to delete instance:", e);
        } finally {
            setLoading(false);
        }
    };

    return (
        <ModalShell
            isOpen={isOpen}
            onClose={onClose}
            title="Delete Instance"
            subtitle={instance?.name}
            icon={I.trash}
            accent="#ff6b6b"
            size="md"
            footer={
                <>
                    <FooterBtn onClick={onClose}>Cancel</FooterBtn>
                    <FooterBtn primary accent="#ff6b6b" onClick={handleDelete} loading={loading}>
                        Delete
                    </FooterBtn>
                </>
            }
        >
            <p className="text-sm text-ink-2 leading-relaxed">
                This will permanently delete the instance directory and all its files
                including worlds, mods, and configuration. This action cannot be undone.
            </p>
        </ModalShell>
    );
}
