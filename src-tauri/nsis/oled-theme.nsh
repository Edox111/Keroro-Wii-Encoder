!ifndef KERORO_OLED_THEME
!define KERORO_OLED_THEME

!define MUI_BGCOLOR "000000"
!define MUI_TEXTCOLOR "FFFFFF"
!define MUI_LICENSEPAGE_BGCOLOR "000000"
!define MUI_INSTFILESPAGE_COLORS "FFFFFF 000000"
!define MUI_INSTFILESPAGE_PROGRESSBAR "colored"
!define MUI_CUSTOMFUNCTION_GUIINIT KeroroGUIInit
!define MUI_CUSTOMFUNCTION_UNGUIINIT un.KeroroGUIInit

; Page windows are created lazily: recolor their controls each time a page appears.
!macro KeroroThemeFunctions PREFIX
Function ${PREFIX}KeroroColorTree
  Exch $0
  Push $1
  Push $2
  ; Windows themes ignore custom text colors on checkable buttons. Disable
  ; them only for these controls; keep native styling on the frame/navigation.
  System::Call 'user32::GetClassNameW(p r0, w .r2, i ${NSIS_MAX_STRLEN})'
  ${If} $2 == "Button"
    System::Call 'user32::GetWindowLongW(p r0, i -16) i .r2'
    IntOp $2 $2 & 0x0F
    ${If} $2 == 2
    ${OrIf} $2 == 3
    ${OrIf} $2 == 4
    ${OrIf} $2 == 5
    ${OrIf} $2 == 6
    ${OrIf} $2 == 9
      System::Call 'uxtheme::SetWindowTheme(p r0, w "", w "")'
    ${EndIf}
  ${EndIf}
  SetCtlColors $0 FFFFFF 000000
  System::Call 'user32::GetWindow(p r0, i 5) p .r1'
  loop:
    StrCmp $1 0 done
    Push $1
    Call ${PREFIX}KeroroColorTree
    System::Call 'user32::GetWindow(p r1, i 2) p .r1'
    Goto loop
  done:
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function ${PREFIX}KeroroPageShow
  Push $HWNDPARENT
  Call ${PREFIX}KeroroColorTree
FunctionEnd

Function ${PREFIX}KeroroGUIInit
  ; Windows controls UAC and native message boxes. Request a dark title bar
  ; where supported; failure on older Windows does not affect installation.
  System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 20, *i 1, i 4)'
  Call ${PREFIX}KeroroPageShow
FunctionEnd
!macroend

!insertmacro KeroroThemeFunctions ""
!insertmacro KeroroThemeFunctions "un."
!endif
