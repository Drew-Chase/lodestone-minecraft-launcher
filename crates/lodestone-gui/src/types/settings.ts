export interface Settings {
    instanceDir: string;
    startupBehavior: string;
    onGameLaunch: string;
    afterGameExits: string;
    autoUpdate: boolean;
    betaChannel: boolean;
    autoUpdateGames: boolean;
    concurrentDownloads: number;
    maxMemoryMb: number;
    defaultJavaPath: string;
    jvmArguments: string;
    accentColor: string;
    theme: string;
    animations: boolean;
    particles: boolean;
    glass: boolean;
    aurora: boolean;
    reduceMotion: boolean;
    font: string;
    maxConcurrentDownloads: number;
    modSourcePriority: string;
    assetCdn: string;
    connectionTimeout: number;
    useSystemProxy: boolean;
    customProxyUrl: string;
    crashReports: boolean;
    usageStats: boolean;
    performanceDiagnostics: boolean;
    filesystemAccess: boolean;
    networkAccess: boolean;
    hardwareAccess: boolean;
}

export interface DetectedJava {
    label: string;
    version: string;
    path: string;
    source: "system" | "managed";
}

export interface MojangRuntime {
    component: string;
    javaVersion: number;
    label: string;
    installed: boolean;
}

export interface JavaInstallProgress {
    component: string;
    filesDownloaded: number;
    filesTotal: number;
}
