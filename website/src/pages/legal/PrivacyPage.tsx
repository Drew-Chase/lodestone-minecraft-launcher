import LegalPage from "../../components/legal/LegalPage.tsx";

export default function PrivacyPage() {
    return (
        <LegalPage
            title="Privacy Policy"
            paragraphs={[
                "This page describes what data the Lodestone website and account service handle.",
                "An account stores your email address, an optional display name, and the time it was created.",
                "Passwords are stored only as hashes. Requests are checked against a breached-password list by a partial hash.",
            ]}
        />
    );
}
