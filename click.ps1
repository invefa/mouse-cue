param(
    [Parameter(Mandatory = $true)][int]$X,
    [Parameter(Mandatory = $true)][int]$Y,
    [string]$Title = '鼠标操作',
    [string]$Detail = '',
    [int]$Countdown = 4,
    [ValidateSet('left','double','right')][string]$Button = 'left',
    [int]$SettleMs = 6000,
    [int]$Hwnd = 0
)

$ErrorActionPreference = 'Stop'
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class CueClick {
  [DllImport("user32.dll", SetLastError=true)] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  public struct POINT { public int x; public int y; }
}
"@

[void][CueClick]::SetProcessDPIAware()
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $here 'target\release\mouse-cue.exe'
if (-not (Test-Path $exe)) { throw "mouse-cue.exe not found: $exe" }

$plan = Join-Path $here 'run.plan.txt'
$out = Join-Path $here 'run.out.json'
$err = Join-Path $here 'run.err.txt'
$lines = @(
    "title=$Title"
    "detail=$Detail"
    "countdown=$Countdown"
    "move=$X,$Y,target"
    "wait=$SettleMs"
)
$planText = ($lines -join "`n")
[System.IO.File]::WriteAllText($plan, $planText, (New-Object System.Text.UTF8Encoding($false)))

$proc = Start-Process -FilePath $exe -ArgumentList $plan -PassThru -RedirectStandardOutput $out -RedirectStandardError $err -WindowStyle Hidden
Start-Sleep -Milliseconds (($Countdown * 1000) + 400)

if ($proc.HasExited) {
    $code = $proc.ExitCode
    if ($code -eq 2) { Write-Output '操作失败: 用户取消'; exit 2 }
    Write-Output "预约进程提前退出 code=$code"
    Get-Content $err -Raw -ErrorAction SilentlyContinue | Write-Output
    exit 1
}

if ($Hwnd -ne 0) { [void][CueClick]::SetForegroundWindow([IntPtr]$Hwnd); Start-Sleep -Milliseconds 250 }
$pos = New-Object CueClick+POINT
[void][CueClick]::SetCursorPos($X - 1, $Y)
Start-Sleep -Milliseconds 15
[void][CueClick]::SetCursorPos($X, $Y)
Start-Sleep -Milliseconds 30
[void][CueClick]::GetCursorPos([ref]$pos)

if ($pos.x -ne $X -or $pos.y -ne $Y) {
    [Console]::Error.WriteLine("cursor missed want=($X,$Y) at=($($pos.x),$($pos.y))")
    $proc.WaitForExit(4000) | Out-Null
    exit 3
}

$down = 0x0002
$up = 0x0004
if ($Button -eq 'right') { $down = 0x0008; $up = 0x0010 }
[CueClick]::mouse_event($down, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 80
[CueClick]::mouse_event($up, 0, 0, 0, [UIntPtr]::Zero)
if ($Button -eq 'double') {
    Start-Sleep -Milliseconds 60
    [CueClick]::mouse_event($down, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [CueClick]::mouse_event($up, 0, 0, 0, [UIntPtr]::Zero)
}

$proc.WaitForExit(($SettleMs + 3000)) | Out-Null
Write-Output "click ok ($X,$Y) $Button"
Write-Output ("cue: " + ((Get-Content $out -Raw -ErrorAction SilentlyContinue) -replace "`r?`n"," "))
exit $proc.ExitCode


