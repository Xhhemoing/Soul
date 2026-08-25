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
  ; Tauri runs this hook before the uninstaller removes any file, so it is the
  ; last place that can still say no. $INSTDIR here is whatever the uninstaller
  ; was written with, not what PREINSTALL above chose: an installer built before
  ; that hook existed, a restored previous install location, or a Tauri default
  ; that drifts back can all hand us the data directory. One RMDir there takes
  ; keys.dpapi with it, and the DEK it wraps exists nowhere else — the library
  ; would be unreadable forever, so refusing to uninstall is the cheap outcome.
  ; Abort rather than a dialog: `/S` uninstall (scripts/install-smoke.ps1) has
  ; nobody to answer a MessageBox and would hang waiting.
  StrCmp $INSTDIR "$LOCALAPPDATA\Soul" soul_preuninstall_is_data_dir 0
  StrCmp $INSTDIR "$LOCALAPPDATA\Soul\" soul_preuninstall_is_data_dir 0
  ; $LOCALAPPDATA\${PRODUCTNAME} is the Tauri 2 currentUser default, which is
  ; the same folder spelled the way a future bundler would spell it.
  StrCmp $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}" soul_preuninstall_is_data_dir 0
  StrCmp $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}\" soul_preuninstall_is_data_dir 0
  Goto soul_preuninstall_not_data_dir

  soul_preuninstall_is_data_dir:
  ; Nothing is deleted or repaired on this path. Leaving an install behind is
  ; something the user can undo; deleting the DPAPI blob is not.
  DetailPrint "Soul: $INSTDIR is the data directory (soul.db, keys.dpapi). Uninstall refused."
  Abort

  soul_preuninstall_not_data_dir:
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
!macroend
