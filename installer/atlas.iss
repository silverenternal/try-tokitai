#define AppName "Atlas"
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef SourceDir
  #define SourceDir "..\dist\Atlas-0.1.0-windows-x64"
#endif

[Setup]
AppId={{E80E2937-01E3-4F33-90BD-1A74BD61F6D4}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Atlas
DefaultDirName={autopf}\Atlas
DefaultGroupName=Atlas
UninstallDisplayIcon={app}\Atlas.exe
OutputDir=..\dist
OutputBaseFilename=Atlas-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Files]
Source: "{#SourceDir}\Atlas.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\frontend\*"; DestDir: "{app}\frontend"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#SourceDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Atlas"; Filename: "{app}\Atlas.exe"
Name: "{autodesktop}\Atlas"; Filename: "{app}\Atlas.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"

[Run]
Filename: "{app}\Atlas.exe"; Description: "Launch Atlas"; Flags: nowait postinstall skipifsilent
