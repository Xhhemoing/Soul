; NSIS installer hooks for Soul. Tauri includes this file via
; bundle.windows.nsis.installerHooks in tauri.conf.json.
;
; AC-01 / PRODUCT_LOCK: the encrypted database and DPAPI blob live under
; %LOCALAPPDATA%\Soul (soul.db, keys.dpapi). Tauri 2 currentUser NSIS
; defaults to $LOCALAPPDATA\${PRODUCTNAME}, which is the same folder.
; Uninstall would delete soul.exe and RMDir the tree — including user data.
; These hooks force the per-user Programs convention instead.

!macro NSIS_HOOK_PREINSTALL
  ; Unconditional: even if RestorePreviousInstallLocation pointed at the data
  ; directory, never ship binaries into %LOCALAPPDATA%\Soul.
  StrCpy $INSTDIR "$LOCALAPPDATA\Programs\Soul"
  CreateDirectory "$INSTDIR"
  SetOutPath $INSTDIR
!macroend

!macro NSIS_HOOK_POSTINSTALL
!macroend

!macro NSIS_HOOK_PREUNINSTALL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
!macroend
