Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"

!define APP_VERSION "0.15.10"

Name "CAA Writer"
OutFile "CAA-Writer_${APP_VERSION}_windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\CAA Writer"
InstallDirRegKey HKCU "Software\CAAWriter" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetCompressorDictSize 64

!define MUI_ICON "icon.ico"
!define MUI_UNICON "icon.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\caa-writer.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Avvia CAA Writer"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE.txt"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Italian"
!insertmacro MUI_LANGUAGE "English"

!define WEBVIEW2_GUID "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"

Section "CAA Writer" SEC_MAIN
  SetOutPath "$INSTDIR"
  File "caa-writer.exe"
  File "icon.ico"
  File "MicrosoftEdgeWebview2Setup.exe"
  File "vc_redist.x64.exe"
  SetOutPath "$INSTDIR"
  File /r "resources"

  ClearErrors
  ReadRegStr $0 HKCU "Software\Microsoft\EdgeUpdate\Clients\${WEBVIEW2_GUID}" "pv"
  ReadRegStr $1 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${WEBVIEW2_GUID}" "pv"
  ReadRegStr $2 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2_GUID}" "pv"
  StrCmp $0 "" 0 webview_ok
  StrCmp $1 "" 0 webview_ok
  StrCmp $2 "" 0 webview_ok
  DetailPrint "Installazione del runtime Microsoft Edge WebView2…"
  ExecWait '"$INSTDIR\MicrosoftEdgeWebview2Setup.exe" /silent /install'
webview_ok:
  Delete "$INSTDIR\MicrosoftEdgeWebview2Setup.exe"

  ClearErrors
  ReadRegDWORD $0 HKLM "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" "Installed"
  StrCmp $0 "1" vcredist_ok
  DetailPrint "Installazione del runtime Visual C++…"
  ExecWait '"$INSTDIR\vc_redist.x64.exe" /quiet /norestart'
vcredist_ok:
  Delete "$INSTDIR\vc_redist.x64.exe"

  CreateDirectory "$SMPROGRAMS\CAA Writer"
  CreateShortcut "$SMPROGRAMS\CAA Writer\CAA Writer.lnk" "$INSTDIR\caa-writer.exe" "" "$INSTDIR\icon.ico" 0
  CreateShortcut "$SMPROGRAMS\CAA Writer\Disinstalla CAA Writer.lnk" "$INSTDIR\uninstall.exe"
  CreateShortcut "$DESKTOP\CAA Writer.lnk" "$INSTDIR\caa-writer.exe" "" "$INSTDIR\icon.ico" 0
  WriteUninstaller "$INSTDIR\uninstall.exe"

  WriteRegStr HKCU "Software\Classes\.caadoc" "" "CAAWriter.Document"
  WriteRegStr HKCU "Software\Classes\.caadoc" "Content Type" "application/x-caadoc"
  WriteRegStr HKCU "Software\Classes\CAAWriter.Document" "" "Documento CAA Writer"
  WriteRegStr HKCU "Software\Classes\CAAWriter.Document\DefaultIcon" "" "$INSTDIR\caa-writer.exe,0"
  WriteRegStr HKCU "Software\Classes\CAAWriter.Document\shell\open\command" "" '"$INSTDIR\caa-writer.exe" "%1"'
  WriteRegStr HKCU "Software\Classes\.caaplay" "" "CAAWriter.Player"
  WriteRegStr HKCU "Software\Classes\.caaplay" "Content Type" "application/x-caaplay"
  WriteRegStr HKCU "Software\Classes\CAAWriter.Player" "" "Documento CAA Writer (player)"
  WriteRegStr HKCU "Software\Classes\CAAWriter.Player\DefaultIcon" "" "$INSTDIR\caa-writer.exe,0"
  WriteRegStr HKCU "Software\Classes\CAAWriter.Player\shell\open\command" "" '"$INSTDIR\caa-writer.exe" "%1"'
  WriteRegStr HKCU "Software\CAAWriter" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\CAAWriter" "Version" "${APP_VERSION}"

  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CAAWriter" "DisplayName" "CAA Writer"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CAAWriter" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CAAWriter" "Publisher" "CAA Writer contributors"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CAAWriter" "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CAAWriter" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CAAWriter" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CAAWriter" "NoModify" "1"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CAAWriter" "NoRepair" "1"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CAAWriter" "EstimatedSize" $0
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\caa-writer.exe"
  Delete "$INSTDIR\icon.ico"
  RMDir /r "$INSTDIR\resources"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$DESKTOP\CAA Writer.lnk"
  RMDir /r "$SMPROGRAMS\CAA Writer"
  DeleteRegKey HKCU "Software\Classes\.caadoc"
  DeleteRegKey HKCU "Software\Classes\CAAWriter.Document"
  DeleteRegKey HKCU "Software\Classes\.caaplay"
  DeleteRegKey HKCU "Software\Classes\CAAWriter.Player"
  DeleteRegKey HKCU "Software\CAAWriter"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CAAWriter"
SectionEnd
