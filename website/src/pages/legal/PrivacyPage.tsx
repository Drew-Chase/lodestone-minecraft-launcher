import LegalPage from "../../components/legal/LegalPage.tsx";
import privacy from "../../content/legal/privacy-policy.md?raw";

export default function PrivacyPage() {
    return <LegalPage markdown={privacy}/>;
}
