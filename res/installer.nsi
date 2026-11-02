; Lodestone Minecraft Launcher: the Windows installer.
;
; Build from the repository root (CI passes absolute paths):
;   makensis /DAPP_VERSION=0.1.0 /DBIN_DIR=C:\path\to\target\release /DOUT_FILE=C:\path\to\Lodestone-setup.exe res\installer.nsi
; Without the defines it builds from ..\target\release and writes Lodestone-setup.exe there. NSIS takes every relative
; path from this script's folder, so these defaults do not depend on where makensis is run from.
;
; The installer writes to Program Files, so it asks for administrator rights. `/S` runs it silently; the launcher's
; own updater does that.

Unicode true

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

!ifndef APP_VERSION
  !define APP_VERSION "0.1.0"
!endif
!ifndef BIN_DIR
  !define BIN_DIR "..\target\release"
!endif
!ifndef OUT_FILE
  !define OUT_FILE "..\target\release\Lodestone-setup.exe"
!endif

!define APP_NAME      "Lodestone"
!define APP_PRODUCT   "Lodestone Minecraft Launcher"
!define APP_PUBLISHER "Drew Chase"
!define APP_EXE       "lodestone_launcher.exe"
!define APP_ICON      "icon\icon.ico"
!define UNINST_KEY    "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"

Name "${APP_PRODUCT}"
OutFile "${OUT_FILE}"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
InstallDir "$PROGRAMFILES64\${APP_NAME}"
InstallDirRegKey HKLM "Software\${APP_NAME}" "InstallPath"

VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey "ProductName" "${APP_PRODUCT}"
VIAddVersionKey "CompanyName" "${APP_PUBLISHER}"
VIAddVersionKey "LegalCopyright" "Copyright (c) 2026 ${APP_PUBLISHER}"
VIAddVersionKey "FileDescription" "${APP_PRODUCT} Installer"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"

!define MUI_ICON "${APP_ICON}"
!define MUI_UNICON "${APP_ICON}"
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ${APP_PRODUCT}"
!define MUI_FINISHPAGE_RUN_FUNCTION "LaunchLauncher"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "${APP_PRODUCT} needs 64-bit Windows."
    Abort
  ${EndIf}
  SetRegView 64
  SetShellVarContext all
FunctionEnd

; Starts the launcher through Explorer, which runs it as the signed-in user. This installer runs elevated, and the
; launcher should not.
Function LaunchLauncher
  Exec 'explorer.exe "$INSTDIR\${APP_EXE}"'
FunctionEnd

; A silent install has no Finish page, so it relaunches the launcher itself. That covers the updater's installs.
Function .onInstSuccess
  IfSilent 0 +2
    Call LaunchLauncher
FunctionEnd

Section "Install" SecInstall
  ; The updater starts this installer while the launcher runs. Close the launcher gently first, so it can save its
  ; state, then end whatever is still running.
  nsExec::Exec 'taskkill /IM "${APP_EXE}"'
  Sleep 2000
  nsExec::Exec 'taskkill /F /IM "${APP_EXE}"'

  SetOutPath "$INSTDIR"
  File "${BIN_DIR}\${APP_EXE}"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; The launcher has to be able to write to its own folder, so the Users group gets Modify on it and everything in it.
  ; The SID is used because the group's name changes with the Windows language.
  nsExec::Exec 'icacls "$INSTDIR" /grant *S-1-5-32-545:(OI)(CI)M /T /C'

  WriteRegStr HKLM "Software\${APP_NAME}" "InstallPath" "$INSTDIR"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "${APP_PRODUCT}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "${APP_PUBLISHER}"
  WriteRegStr HKLM "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair" 1

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_PRODUCT}.lnk" "$INSTDIR\${APP_EXE}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  SetRegView 64
  SetShellVarContext all

  nsExec::Exec 'taskkill /IM "${APP_EXE}"'
  Sleep 2000
  nsExec::Exec 'taskkill /F /IM "${APP_EXE}"'

  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${APP_NAME}\${APP_PRODUCT}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"

  DeleteRegKey HKLM "${UNINST_KEY}"
  DeleteRegKey HKLM "Software\${APP_NAME}"
  ; The library and settings live in the user's AppData, so they are kept.
SectionEnd
