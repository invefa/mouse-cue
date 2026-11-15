param(
    [Parameter(Mandatory = $true)][int]$X,
    [Parameter(Mandatory = $true)][int]$Y,
    [ValidateSet('left', 'double', 'right')][string]$Button = 'left',
    [int]$HoldMs = 70
)

$work = Split-Path -Parent $MyInvocation.MyCommand.Path
$tmp = Join-Path $work '.tmp'
if (-not (Test-Path $tmp)) { New-Item -ItemType Directory -Path $tmp -Force | Out-Null }
$env:TEMP = $tmp
$env:TMP = $tmp

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Hit {
  [DllImport("user32.dll", SetLastError=true)] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, UIntPtr extra);
  public struct POINT { public int x; public int y; }
}
"@

[void][Hit]::SetCursorPos($X - 1, $Y)
Start-Sleep -Milliseconds 15
[void][Hit]::SetCursorPos($X, $Y)
Start-Sleep -Milliseconds 30

$pos = New-Object Hit+POINT
[void][Hit]::GetCursorPos([ref]$pos)
if ($pos.x -ne $X -or $pos.y -ne $Y) {
    [Console]::Error.WriteLine("hit: cursor not at target want=($X,$Y) at=($($pos.x),$($pos.y))")
    exit 3
}

$down = 0x0002
$up = 0x0004
if ($Button -eq 'right') { $down = 0x0008; $up = 0x0010 }

[Hit]::mouse_event($down, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds $HoldMs
[Hit]::mouse_event($up, 0, 0, 0, [UIntPtr]::Zero)

if ($Button -eq 'double') {
    Start-Sleep -Milliseconds 60
    [Hit]::mouse_event($down, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds $HoldMs
    [Hit]::mouse_event($up, 0, 0, 0, [UIntPtr]::Zero)
}

Write-Output "hit ok ($X,$Y) $Button"
exit 0
