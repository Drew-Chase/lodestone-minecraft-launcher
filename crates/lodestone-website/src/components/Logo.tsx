export default function Logo({size = 30}: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M6 2 H26 L30 6 V26 L26 30 H6 L2 26 V6 Z" fill="none" stroke="#22ff84" strokeWidth="2" strokeLinejoin="round"/>
            <circle cx="16" cy="16" r="8.5" stroke="#22ff84" strokeWidth="1.4" strokeOpacity="0.45" fill="none"/>
            <rect x="15.25" y="5.5" width="1.5" height="2" fill="#22ff84"/>
            <rect x="24.5" y="15.25" width="2" height="1.5" fill="#22ff84" fillOpacity="0.55"/>
            <rect x="15.25" y="24.5" width="1.5" height="2" fill="#22ff84" fillOpacity="0.55"/>
            <rect x="5.5" y="15.25" width="2" height="1.5" fill="#22ff84" fillOpacity="0.55"/>
            <path d="M16 16 L22.5 9.5 L20 16 Z" fill="#22ff84"/>
            <path d="M16 16 L9.5 22.5 L12 16 Z" fill="#22ff84" fillOpacity="0.35"/>
            <circle cx="16" cy="16" r="1.6" fill="#08090a" stroke="#22ff84" strokeWidth="1"/>
        </svg>
    );
}
