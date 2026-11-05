// A form error. It is announced as soon as it appears. Renders nothing when there is no message.
export default function FormAlert({message}: { message: string | null }) {
    if (!message) return null;
    return <p className="acct-alert" role="alert">{message}</p>;
}
