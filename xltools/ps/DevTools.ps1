# Installs vs_BuildTools for Rust development
# https://learn.microsoft.com/en-us/windows/dev-environment/rust/setup

$RustupURL = "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe"
$RustupPath "~/Downloads/rustup-init.exe"
$VSToolsURL = "https://aka.ms/vs/17/release/vs_BuildTools.exe"
$VSToolsPath = "~/Downloads/vs_BuildTools.exe"

function Install-VSBuildTools {
    # M$ Suggests .NET platform development, Desktop Development with C++, and UWP
    # Arguments to installer: https://learn.microsoft.com/en-us/visualstudio/install/use-command-line-parameters-to-install-visual-studio?view=vs-2022
    # List of workloads: https://learn.microsoft.com/en-us/visualstudio/install/workload-component-id-vs-build-tools?view=vs-2022#net-desktop-build-tools
    $WorkLoads = @(
        # .NET Desktop Build Tools
        "--add"
        "Microsoft.VisualStudio.Workload.ManagedDesktopBuildTools"
        # UWP
        "--add"
        "Microsoft.VisualStudio.Workload.UniversalBuildTools"
        # Desktop Development With C++
        "--add"
        "Microsoft.VisualStudio.Workload.VCTools"
        # We don't need to install GIT because Cargo will use its own
    )

    $OtherArgs = @(
        "--includeRecommended"
        "--quiet"
        "--norestart"
        "--force"
    )

    $Args = $OtherArgs
    $Args += $WorkLoads -join " "

    $Result = Start-Process $VSToolsPath -ArgumentList $Args -Wait
    return $Result
}

function Install-Rustup {
    $Result = Start-Process $RustupPath -Wait
    $RustupArgs = "default stable-msvc"
    $Result = Start-Process "rustup" -ArgumentList $RustupArgs -Wait
}

function Download-VSBuildTools {
    Invoke-WebRequest -Uri $VSToolsURL -OutFile $VSToolsPath
}

function Download-Rustup {
    Invoke-WebRequest -Uri $RustupURL -OutFile $RustupPath
}

Download-VSBuildTools
Install-VSBuildTools
Download-Rustup
Install-Rustup