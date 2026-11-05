import {useId} from "react";
import type {InputHTMLAttributes} from "react";

interface FieldProps extends InputHTMLAttributes<HTMLInputElement> {
    label: string;
    /** Help text shown under the field and tied to it for screen readers. */
    hint?: string;
}

// A labelled text input. The label is always visible, and the hint is linked to the input by aria-describedby.
export default function Field({label, hint, id, ...input}: FieldProps) {
    const generated = useId();
    const fieldId = id ?? generated;
    const hintId = `${fieldId}-hint`;

    return (
        <div className="acct-field">
            <label className="acct-label" htmlFor={fieldId}>{label}</label>
            <input
                {...input}
                id={fieldId}
                className="acct-input"
                aria-describedby={hint ? hintId : undefined}
            />
            {hint && <p className="acct-hint" id={hintId}>{hint}</p>}
        </div>
    );
}
