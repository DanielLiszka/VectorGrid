$script:BenchmarkLockLeaseSchema = "ta_desktop_demo_benchmark_lock_active_lease_v1"
$script:BenchmarkLockDefaultMutexName = "Local\ta_desktop_demo_benchmark_gate"
$script:BenchmarkLockEnvironmentNames = @(
  "TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD",
  "TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID",
  "TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_STARTED_AT",
  "TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT",
  "TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN",
  "TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN_SHA256",
  "TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_PATH",
  "TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_SHA256",
  "TA_DESKTOP_DEMO_BENCHMARK_LOCK_MUTEX_NAME"
)

function Initialize-BenchmarkLockNative {
  if ($null -ne ("VectorGrid.BenchmarkLockNative" -as [type])) {
    return
  }

  Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using Microsoft.Win32.SafeHandles;

namespace VectorGrid
{
    public static class BenchmarkLockNative
    {
        private const uint GENERIC_READ = 0x80000000;
        private const uint GENERIC_WRITE = 0x40000000;
        private const uint FILE_SHARE_READ = 0x00000001;
        private const uint OPEN_ALWAYS = 4;
        private const uint FILE_ATTRIBUTE_NORMAL = 0x00000080;
        private const uint FILE_FLAG_OPEN_REPARSE_POINT = 0x00200000;
        private const uint TH32CS_SNAPPROCESS = 0x00000002;
        private const int ERROR_MORE_DATA = 234;

        [StructLayout(LayoutKind.Sequential)]
        private struct ByHandleFileInformation
        {
            public uint FileAttributes;
            public System.Runtime.InteropServices.ComTypes.FILETIME CreationTime;
            public System.Runtime.InteropServices.ComTypes.FILETIME LastAccessTime;
            public System.Runtime.InteropServices.ComTypes.FILETIME LastWriteTime;
            public uint VolumeSerialNumber;
            public uint FileSizeHigh;
            public uint FileSizeLow;
            public uint NumberOfLinks;
            public uint FileIndexHigh;
            public uint FileIndexLow;
        }

        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        private struct ProcessEntry32
        {
            public uint Size;
            public uint UsageCount;
            public uint ProcessId;
            public IntPtr DefaultHeapId;
            public uint ModuleId;
            public uint ThreadCount;
            public uint ParentProcessId;
            public int BasePriority;
            public uint Flags;

            [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)]
            public string ExecutableFile;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct RestartManagerUniqueProcess
        {
            public int ProcessId;
            public System.Runtime.InteropServices.ComTypes.FILETIME ProcessStartTime;
        }

        private enum RestartManagerApplicationType
        {
            Unknown = 0,
            MainWindow = 1,
            OtherWindow = 2,
            Service = 3,
            Explorer = 4,
            Console = 5,
            Critical = 1000
        }

        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        private struct RestartManagerProcessInfo
        {
            public RestartManagerUniqueProcess Process;

            [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)]
            public string ApplicationName;

            [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 64)]
            public string ServiceShortName;

            public RestartManagerApplicationType ApplicationType;
            public uint ApplicationStatus;
            public uint TerminalServicesSessionId;

            [MarshalAs(UnmanagedType.Bool)]
            public bool Restartable;
        }

        public sealed class LockingProcessIdentity
        {
            public int ProcessId { get; private set; }
            public string StartedAt { get; private set; }

            internal LockingProcessIdentity(int processId, string startedAt)
            {
                ProcessId = processId;
                StartedAt = startedAt;
            }
        }

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern SafeFileHandle CreateFile(
            string fileName,
            uint desiredAccess,
            uint shareMode,
            IntPtr securityAttributes,
            uint creationDisposition,
            uint flagsAndAttributes,
            IntPtr templateFile);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetFileInformationByHandle(
            SafeFileHandle file,
            out ByHandleFileInformation information);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern IntPtr CreateToolhelp32Snapshot(uint flags, uint processId);

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool Process32FirstW(
            IntPtr snapshot,
            ref ProcessEntry32 entry);

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool Process32NextW(
            IntPtr snapshot,
            ref ProcessEntry32 entry);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CloseHandle(IntPtr handle);

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern uint GetFinalPathNameByHandle(
            SafeFileHandle file,
            StringBuilder filePath,
            uint filePathLength,
            uint flags);

        [DllImport("rstrtmgr.dll", CharSet = CharSet.Unicode)]
        private static extern int RmStartSession(
            out uint sessionHandle,
            int sessionFlags,
            string sessionKey);

        [DllImport("rstrtmgr.dll", CharSet = CharSet.Unicode)]
        private static extern int RmRegisterResources(
            uint sessionHandle,
            uint fileCount,
            string[] fileNames,
            uint applicationCount,
            RestartManagerUniqueProcess[] applications,
            uint serviceCount,
            string[] serviceNames);

        [DllImport("rstrtmgr.dll")]
        private static extern int RmGetList(
            uint sessionHandle,
            out uint processInfoNeeded,
            ref uint processInfoCount,
            [In, Out] RestartManagerProcessInfo[] affectedApplications,
            ref uint rebootReasons);

        [DllImport("rstrtmgr.dll")]
        private static extern int RmEndSession(uint sessionHandle);

        public static string GetFileIdentity(SafeFileHandle handle)
        {
            ByHandleFileInformation information;
            if (!GetFileInformationByHandle(handle, out information))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }

            return String.Format(
                "{0:x8}:{1:x8}{2:x8}",
                information.VolumeSerialNumber,
                information.FileIndexHigh,
                information.FileIndexLow);
        }

        public static uint GetNumberOfLinks(SafeFileHandle handle)
        {
            ByHandleFileInformation information;
            if (!GetFileInformationByHandle(handle, out information))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }

            return information.NumberOfLinks;
        }

        public static uint GetFileAttributes(SafeFileHandle handle)
        {
            ByHandleFileInformation information;
            if (!GetFileInformationByHandle(handle, out information))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }

            return information.FileAttributes;
        }

        public static string GetFinalPath(SafeFileHandle handle)
        {
            StringBuilder path = new StringBuilder(32768);
            uint length = GetFinalPathNameByHandle(
                handle,
                path,
                (uint)path.Capacity,
                0);
            if (length == 0)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            if (length >= path.Capacity)
            {
                path = new StringBuilder(checked((int)length + 1));
                length = GetFinalPathNameByHandle(
                    handle,
                    path,
                    (uint)path.Capacity,
                    0);
                if (length == 0 || length >= path.Capacity)
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
            }

            return path.ToString();
        }

        public static SafeFileHandle OpenActiveLease(string path)
        {
            SafeFileHandle handle = CreateFile(
                path,
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ,
                IntPtr.Zero,
                OPEN_ALWAYS,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
                IntPtr.Zero);
            if (handle.IsInvalid)
            {
                int error = Marshal.GetLastWin32Error();
                handle.Dispose();
                throw new Win32Exception(error);
            }

            return handle;
        }

        private static LockingProcessIdentity[] GetLockingProcesses(string path)
        {
            uint sessionHandle;
            string sessionKey = Guid.NewGuid().ToString("N");
            int result = RmStartSession(out sessionHandle, 0, sessionKey);
            if (result != 0)
            {
                throw new Win32Exception(result);
            }

            try
            {
                result = RmRegisterResources(
                    sessionHandle,
                    1,
                    new string[] { path },
                    0,
                    null,
                    0,
                    null);
                if (result != 0)
                {
                    throw new Win32Exception(result);
                }

                uint needed;
                uint count = 0;
                uint rebootReasons = 0;
                result = RmGetList(
                    sessionHandle,
                    out needed,
                    ref count,
                    null,
                    ref rebootReasons);
                if (result == 0 && needed == 0)
                {
                    return new LockingProcessIdentity[0];
                }
                if (result != ERROR_MORE_DATA)
                {
                    throw new Win32Exception(result);
                }
                if (needed > 4096)
                {
                    throw new InvalidOperationException(
                        "Restart Manager returned an unreasonable locking-process count.");
                }

                RestartManagerProcessInfo[] information =
                    new RestartManagerProcessInfo[needed];
                count = needed;
                result = RmGetList(
                    sessionHandle,
                    out needed,
                    ref count,
                    information,
                    ref rebootReasons);
                if (result != 0)
                {
                    throw new Win32Exception(result);
                }

                List<LockingProcessIdentity> identities =
                    new List<LockingProcessIdentity>();
                for (int index = 0; index < count; index++)
                {
                    long fileTime =
                        ((long)information[index].Process.ProcessStartTime.dwHighDateTime << 32) |
                        (uint)information[index].Process.ProcessStartTime.dwLowDateTime;
                    string startedAt = DateTime.FromFileTimeUtc(fileTime).ToString(
                        "o",
                        System.Globalization.CultureInfo.InvariantCulture);
                    identities.Add(new LockingProcessIdentity(
                        information[index].Process.ProcessId,
                        startedAt));
                }

                return identities.ToArray();
            }
            finally
            {
                RmEndSession(sessionHandle);
            }
        }

        public static LockingProcessIdentity[] GetLockingProcessesBounded(
            string path,
            int timeoutMilliseconds)
        {
            Task<LockingProcessIdentity[]> task = Task.Factory.StartNew(
                () => GetLockingProcesses(path));
            if (!task.Wait(timeoutMilliseconds))
            {
                throw new TimeoutException(
                    "Restart Manager locking-process query timed out.");
            }

            return task.Result;
        }

        public static int[] GetAncestorProcessIds(int processId)
        {
            IntPtr snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if (snapshot == new IntPtr(-1))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }

            try
            {
                Dictionary<int, int> parents = new Dictionary<int, int>();
                ProcessEntry32 entry = new ProcessEntry32();
                entry.Size = (uint)Marshal.SizeOf(typeof(ProcessEntry32));
                if (!Process32FirstW(snapshot, ref entry))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }

                while (true)
                {
                    parents[(int)entry.ProcessId] = (int)entry.ParentProcessId;
                    entry.Size = (uint)Marshal.SizeOf(typeof(ProcessEntry32));
                    if (!Process32NextW(snapshot, ref entry))
                    {
                        int error = Marshal.GetLastWin32Error();
                        if (error != 18)
                        {
                            throw new Win32Exception(error);
                        }
                        break;
                    }
                }

                List<int> ancestors = new List<int>();
                HashSet<int> visited = new HashSet<int>();
                int current = processId;
                while (parents.ContainsKey(current))
                {
                    int parent = parents[current];
                    if (parent <= 0 || parent == current || !visited.Add(parent))
                    {
                        break;
                    }
                    ancestors.Add(parent);
                    current = parent;
                }

                return ancestors.ToArray();
            }
            finally
            {
                CloseHandle(snapshot);
            }
        }
    }
}
'@
}

function Get-BenchmarkLockSha256Bytes {
  param([Parameter(Mandatory = $true)][byte[]]$Bytes)

  $sha256 = [System.Security.Cryptography.SHA256]::Create()
  try {
    $hash = $sha256.ComputeHash($Bytes)
    return ([System.BitConverter]::ToString($hash)).Replace("-", "").ToLowerInvariant()
  } finally {
    $sha256.Dispose()
  }
}

function Get-BenchmarkLockSha256Text {
  param([Parameter(Mandatory = $true)][string]$Value)

  $bytes = [System.Text.UTF8Encoding]::new($false).GetBytes($Value)
  return Get-BenchmarkLockSha256Bytes -Bytes $bytes
}

function Get-BenchmarkLockProcessStartIdentity {
  param([Parameter(Mandatory = $true)][int]$ProcessId)

  $process = Get-Process -Id $ProcessId -ErrorAction Stop
  try {
    return $process.StartTime.ToUniversalTime().ToString(
      "o",
      [System.Globalization.CultureInfo]::InvariantCulture
    )
  } finally {
    $process.Dispose()
  }
}

function Get-BenchmarkLockLeaseDirectory {
  $repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
  $current = $repoRoot
  foreach ($segment in @("output", "qa", "benchmark-lock")) {
    $current = [System.IO.Path]::GetFullPath((Join-Path $current $segment))
    if (Test-Path -LiteralPath $current) {
      $item = Get-Item -LiteralPath $current -Force -ErrorAction Stop
      if (
        -not $item.PSIsContainer -or
        (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0)
      ) {
        throw "Benchmark-lock lease directory component is not an ordinary directory: $current"
      }
    } else {
      [System.IO.Directory]::CreateDirectory($current) | Out-Null
      $item = Get-Item -LiteralPath $current -Force -ErrorAction Stop
      if (
        -not $item.PSIsContainer -or
        (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0)
      ) {
        throw "Benchmark-lock lease directory creation did not produce an ordinary directory: $current"
      }
    }
  }

  return $current
}

function Assert-BenchmarkLockLeasePath {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [switch]$MustExist
  )

  if (-not [System.IO.Path]::IsPathRooted($Path)) {
    throw "Benchmark-lock lease path must be absolute."
  }
  $leaseDirectory = Get-BenchmarkLockLeaseDirectory
  $resolvedPath = [System.IO.Path]::GetFullPath($Path)
  $canonicalPath = [System.IO.Path]::GetFullPath(
    (Join-Path $leaseDirectory "active.json")
  )
  if (
    -not $resolvedPath.Equals(
      $canonicalPath,
      [System.StringComparison]::OrdinalIgnoreCase
    )
  ) {
    throw "Benchmark-lock lease path is not the fixed canonical active lease."
  }

  if (Test-Path -LiteralPath $resolvedPath) {
    $item = Get-Item -LiteralPath $resolvedPath -Force -ErrorAction Stop
    if (
      $item.PSIsContainer -or
      (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0)
    ) {
      throw "Benchmark-lock lease is not an ordinary file."
    }
  } elseif ($MustExist) {
    throw "Benchmark-lock canonical active lease does not exist."
  }

  return $resolvedPath
}

function Get-BenchmarkLockEnvironmentSnapshot {
  $snapshot = [ordered]@{}
  foreach ($name in $script:BenchmarkLockEnvironmentNames) {
    $item = Get-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
    $snapshot[$name] = [pscustomobject]@{
      exists = $null -ne $item
      value = if ($null -eq $item) { $null } else { [string]$item.Value }
    }
  }
  return $snapshot
}

function Restore-BenchmarkLockEnvironment {
  param([Parameter(Mandatory = $true)]$Snapshot)

  foreach ($name in $script:BenchmarkLockEnvironmentNames) {
    $entry = $Snapshot[$name]
    if ($null -ne $entry -and [bool]$entry.exists) {
      Set-Item -LiteralPath "Env:$name" -Value ([string]$entry.value)
    } else {
      Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
    }
  }
}

function Restore-BenchmarkLockEnvValue {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [string]$Value
  )

  if ([string]::IsNullOrWhiteSpace($Value)) {
    Remove-Item "Env:$Name" -ErrorAction SilentlyContinue
  } else {
    Set-Item "Env:$Name" -Value $Value
  }
}

function Assert-BenchmarkLockHandlePath {
  param(
    [Parameter(Mandatory = $true)]
    [Microsoft.Win32.SafeHandles.SafeFileHandle]$Handle,
    [Parameter(Mandatory = $true)]
    [string]$ExpectedPath
  )

  Initialize-BenchmarkLockNative
  $finalPath = [VectorGrid.BenchmarkLockNative]::GetFinalPath($Handle)
  if ($finalPath.StartsWith("\\?\UNC\", [System.StringComparison]::OrdinalIgnoreCase)) {
    $finalPath = "\\" + $finalPath.Substring(8)
  } elseif ($finalPath.StartsWith("\\?\", [System.StringComparison]::OrdinalIgnoreCase)) {
    $finalPath = $finalPath.Substring(4)
  }
  $resolvedFinalPath = [System.IO.Path]::GetFullPath($finalPath)
  $resolvedExpectedPath = [System.IO.Path]::GetFullPath($ExpectedPath)
  if (
    -not $resolvedFinalPath.Equals(
      $resolvedExpectedPath,
      [System.StringComparison]::OrdinalIgnoreCase
    )
  ) {
    throw "Benchmark-lock opened handle resolves outside the fixed canonical active lease: $resolvedFinalPath"
  }
}

function Read-BenchmarkLockLease {
  param([Parameter(Mandatory = $true)][string]$Path)

  Initialize-BenchmarkLockNative
  $resolvedPath = Assert-BenchmarkLockLeasePath -Path $Path -MustExist
  $stream = [System.IO.FileStream]::new(
    $resolvedPath,
    [System.IO.FileMode]::Open,
    [System.IO.FileAccess]::Read,
    [System.IO.FileShare]::ReadWrite,
    4096,
    [System.IO.FileOptions]::SequentialScan
  )
  try {
    Assert-BenchmarkLockHandlePath `
      -Handle $stream.SafeFileHandle `
      -ExpectedPath $resolvedPath
    $identity = [VectorGrid.BenchmarkLockNative]::GetFileIdentity($stream.SafeFileHandle)
    $linkCount = [VectorGrid.BenchmarkLockNative]::GetNumberOfLinks($stream.SafeFileHandle)
    if ($linkCount -ne 1) {
      throw "Benchmark-lock lease must have exactly one hard-link name."
    }
    if ($stream.Length -le 0 -or $stream.Length -gt 65536) {
      throw "Benchmark-lock lease size is outside the accepted range."
    }

    $memory = [System.IO.MemoryStream]::new()
    try {
      $stream.CopyTo($memory)
      $bytes = $memory.ToArray()
    } finally {
      $memory.Dispose()
    }
  } finally {
    $stream.Dispose()
  }

  $text = [System.Text.UTF8Encoding]::new($false, $true).GetString($bytes)
  try {
    $payload = $text | ConvertFrom-Json -ErrorAction Stop
  } catch {
    throw "Benchmark-lock lease is not valid UTF-8 JSON: $($_.Exception.Message)"
  }

  return [pscustomobject]@{
    path = $resolvedPath
    bytes = $bytes
    sha256 = Get-BenchmarkLockSha256Bytes -Bytes $bytes
    identity = $identity
    link_count = [int]$linkCount
    payload = $payload
  }
}

function Get-BenchmarkLockLeasePathIdentity {
  param([Parameter(Mandatory = $true)][string]$Path)

  Initialize-BenchmarkLockNative
  $resolvedPath = Assert-BenchmarkLockLeasePath -Path $Path -MustExist
  $stream = [System.IO.FileStream]::new(
    $resolvedPath,
    [System.IO.FileMode]::Open,
    [System.IO.FileAccess]::Read,
    [System.IO.FileShare]::ReadWrite
  )
  try {
    Assert-BenchmarkLockHandlePath `
      -Handle $stream.SafeFileHandle `
      -ExpectedPath $resolvedPath
    $attributes = [VectorGrid.BenchmarkLockNative]::GetFileAttributes(
      $stream.SafeFileHandle
    )
    if (
      ($attributes -band [int][System.IO.FileAttributes]::ReparsePoint) -ne 0
    ) {
      throw "Benchmark-lock canonical path resolves to a reparse point."
    }
    $linkCount = [VectorGrid.BenchmarkLockNative]::GetNumberOfLinks(
      $stream.SafeFileHandle
    )
    if ($linkCount -ne 1) {
      throw "Benchmark-lock canonical path must have exactly one hard-link name."
    }
    return [pscustomobject]@{
      path = $resolvedPath
      identity = [VectorGrid.BenchmarkLockNative]::GetFileIdentity(
        $stream.SafeFileHandle
      )
      link_count = [int]$linkCount
    }
  } finally {
    $stream.Dispose()
  }
}

function Assert-BenchmarkLockLeaseHolder {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][int]$OwnerPid,
    [Parameter(Mandatory = $true)][string]$OwnerStartedAt
  )

  Initialize-BenchmarkLockNative
  try {
    $lockingProcesses = @(
      [VectorGrid.BenchmarkLockNative]::GetLockingProcessesBounded($Path, 5000)
    )
  } catch {
    throw "Benchmark-lock Restart Manager holder query failed closed: $($_.Exception.Message)"
  }

  $matchingOwners = @(
    $lockingProcesses | Where-Object {
      [int]$_.ProcessId -eq $OwnerPid -and
      [string]$_.StartedAt -eq $OwnerStartedAt
    }
  )
  $foreignHolders = @(
    $lockingProcesses | Where-Object {
      [int]$_.ProcessId -ne $OwnerPid -or
      [string]$_.StartedAt -ne $OwnerStartedAt
    }
  )
  if ($matchingOwners.Count -lt 1 -or $foreignHolders.Count -gt 0) {
    $observed = @(
      $lockingProcesses | ForEach-Object {
        "$([int]$_.ProcessId)@$([string]$_.StartedAt)"
      }
    ) -join ", "
    throw "Benchmark-lock fixed lease holder identity does not match claimed owner $OwnerPid@$OwnerStartedAt (observed: $observed)."
  }
}

function Get-BenchmarkLockExceptionWin32Code {
  param([Parameter(Mandatory = $true)][System.Exception]$Exception)

  $current = $Exception
  $code = if ($current -is [System.ComponentModel.Win32Exception]) {
    [int]$current.NativeErrorCode
  } else {
    ([int]$current.HResult -band 0xffff)
  }
  while ($null -ne $current.InnerException) {
    $current = $current.InnerException
    $code = if ($current -is [System.ComponentModel.Win32Exception]) {
      [int]$current.NativeErrorCode
    } else {
      ([int]$current.HResult -band 0xffff)
    }
  }
  return $code
}

function Open-BenchmarkLockActiveLease {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][int]$TimeoutSeconds
  )

  Initialize-BenchmarkLockNative
  $timer = [System.Diagnostics.Stopwatch]::StartNew()
  while ($true) {
    $safeHandle = $null
    try {
      $safeHandle = [VectorGrid.BenchmarkLockNative]::OpenActiveLease($Path)
      $stream = [System.IO.FileStream]::new(
        $safeHandle,
        [System.IO.FileAccess]::ReadWrite,
        4096,
        $false
      )
      $safeHandle = $null
      return $stream
    } catch {
      if ($null -ne $safeHandle) {
        $safeHandle.Dispose()
      }
      $code = Get-BenchmarkLockExceptionWin32Code -Exception $_.Exception
      if ($code -notin @(32, 33)) {
        throw "Could not open the fixed canonical benchmark lease (Win32 error $code): $($_.Exception.Message)"
      }
      if ($timer.Elapsed.TotalSeconds -ge $TimeoutSeconds) {
        throw "Timed out waiting for the fixed canonical benchmark lease after ${TimeoutSeconds}s (Win32 error $code). Another host-wide benchmark authority is still active."
      }
      Start-Sleep -Milliseconds 25
    }
  }
}

function Get-BenchmarkLockLeaseWriteProbeCode {
  param([Parameter(Mandatory = $true)][string]$Path)

  $probe = $null
  try {
    $probe = [System.IO.FileStream]::new(
      $Path,
      [System.IO.FileMode]::Open,
      [System.IO.FileAccess]::Write,
      [System.IO.FileShare]::ReadWrite
    )
    return 0
  } catch {
    return Get-BenchmarkLockExceptionWin32Code -Exception $_.Exception
  } finally {
    if ($null -ne $probe) {
      $probe.Dispose()
    }
  }
}

function Assert-BenchmarkLockMutexHeld {
  param([Parameter(Mandatory = $true)][string]$MutexName)

  $probe = $null
  $acquired = $false
  try {
    try {
      $probe = [System.Threading.Mutex]::OpenExisting($MutexName)
    } catch [System.Threading.WaitHandleCannotBeOpenedException] {
      throw "Benchmark-lock named mutex does not exist."
    }

    try {
      $acquired = $probe.WaitOne(0)
    } catch [System.Threading.AbandonedMutexException] {
      $acquired = $true
    }
    if ($acquired) {
      throw "Benchmark-lock named mutex is not held by the claimed owner."
    }
  } finally {
    if ($acquired -and $null -ne $probe) {
      try {
        $probe.ReleaseMutex() | Out-Null
      } catch {
      }
    }
    if ($null -ne $probe) {
      $probe.Dispose()
    }
  }
}

function Assert-BenchmarkLockInheritedAuthority {
  param(
    [Parameter(Mandatory = $true)][string]$Context,
    [Parameter(Mandatory = $true)][string]$MutexName
  )

  Initialize-BenchmarkLockNative
  $required = @(
    "TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID",
    "TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_STARTED_AT",
    "TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT",
    "TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN",
    "TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN_SHA256",
    "TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_PATH",
    "TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_SHA256",
    "TA_DESKTOP_DEMO_BENCHMARK_LOCK_MUTEX_NAME"
  )
  foreach ($name in $required) {
    $value = [System.Environment]::GetEnvironmentVariable($name, "Process")
    if ([string]::IsNullOrWhiteSpace($value)) {
      throw "Benchmark lock inheritance marker '$name' is missing; refusing to inherit lock for $Context."
    }
  }

  $ownerPid = 0
  if (
    -not [int]::TryParse(
      $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID,
      [ref]$ownerPid
    ) -or
    $ownerPid -le 0
  ) {
    throw "Benchmark lock owner PID is invalid: $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID"
  }
  if ($ownerPid -eq $PID) {
    throw "Benchmark lock inheritance requires a live ancestor, not the current process."
  }

  if ($env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_MUTEX_NAME -ne $MutexName) {
    throw "Benchmark lock inherited mutex name does not match the requested mutex."
  }
  if (
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN -notmatch "^[a-f0-9]{32}$" -or
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN_SHA256 -notmatch "^[a-f0-9]{64}$" -or
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_SHA256 -notmatch "^[a-f0-9]{64}$"
  ) {
    throw "Benchmark lock inheritance hashes or token are malformed."
  }

  $leasePath = Assert-BenchmarkLockLeasePath `
    -Path $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_PATH `
    -MustExist
  $writeProbeCode = Get-BenchmarkLockLeaseWriteProbeCode -Path $leasePath
  if ($writeProbeCode -ne 32) {
    throw "Benchmark-lock canonical lease write probe did not report exact Windows sharing violation 32 (reported $writeProbeCode)."
  }

  $ownerStartedAt = Get-BenchmarkLockProcessStartIdentity -ProcessId $ownerPid
  if ($ownerStartedAt -ne $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_STARTED_AT) {
    throw "Benchmark lock owner PID creation identity does not match the inherited marker."
  }

  $ancestors = @([VectorGrid.BenchmarkLockNative]::GetAncestorProcessIds($PID))
  if (-not ($ancestors -contains $ownerPid)) {
    throw "Benchmark lock owner PID $ownerPid is not a current ancestor of PID $PID."
  }

  $tokenSha256 = Get-BenchmarkLockSha256Text `
    -Value $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN
  if ($tokenSha256 -ne $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN_SHA256) {
    throw "Benchmark lock inherited token hash does not match the token."
  }

  $lease = Read-BenchmarkLockLease -Path $leasePath
  if ($lease.sha256 -ne $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_SHA256) {
    throw "Benchmark lock canonical lease hash does not match the inherited marker."
  }

  $payload = $lease.payload
  $releasedProperty = $payload.PSObject.Properties["released"]
  if (
    "$($payload.schema)" -ne $script:BenchmarkLockLeaseSchema -or
    "$($payload.acquisition_id)" -notmatch "^[a-f0-9]{32}$" -or
    [int]$payload.owner_pid -ne $ownerPid -or
    "$($payload.owner_started_at)" -ne $ownerStartedAt -or
    "$($payload.context)" -ne $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT -or
    "$($payload.token_sha256)" -ne $tokenSha256 -or
    "$($payload.mutex_name)" -ne $MutexName -or
    $null -eq $releasedProperty -or
    [bool]$releasedProperty.Value -ne $false -or
    "$($payload.file_identity)" -ne $lease.identity
  ) {
    throw "Benchmark lock canonical lease contents do not match the inherited authority markers."
  }

  $createdAt = [DateTimeOffset]::MinValue
  if (
    -not [DateTimeOffset]::TryParseExact(
      "$($payload.created_at)",
      "o",
      [System.Globalization.CultureInfo]::InvariantCulture,
      [System.Globalization.DateTimeStyles]::RoundtripKind,
      [ref]$createdAt
    )
  ) {
    throw "Benchmark lock canonical lease creation timestamp is invalid."
  }

  Assert-BenchmarkLockLeaseHolder `
    -Path $leasePath `
    -OwnerPid $ownerPid `
    -OwnerStartedAt $ownerStartedAt
  Assert-BenchmarkLockMutexHeld -MutexName $MutexName

  $finalOwnerStartedAt = Get-BenchmarkLockProcessStartIdentity -ProcessId $ownerPid
  if ($finalOwnerStartedAt -ne $ownerStartedAt) {
    throw "Benchmark lock owner changed while inherited authority was being verified."
  }
  $finalWriteProbeCode = Get-BenchmarkLockLeaseWriteProbeCode -Path $leasePath
  if ($finalWriteProbeCode -ne 32) {
    throw "Benchmark-lock canonical lease stopped proving a live writer before inheritance completed."
  }
  $finalLease = Read-BenchmarkLockLease -Path $leasePath
  if (
    $finalLease.identity -ne $lease.identity -or
    $finalLease.sha256 -ne $lease.sha256
  ) {
    throw "Benchmark-lock canonical lease identity or contents changed while inherited authority was being verified."
  }
  Assert-BenchmarkLockLeaseHolder `
    -Path $leasePath `
    -OwnerPid $ownerPid `
    -OwnerStartedAt $ownerStartedAt

  return [pscustomobject]@{
    acquisition_id = [string]$payload.acquisition_id
    owner_pid = $ownerPid
    owner_started_at = $ownerStartedAt
    context = [string]$payload.context
    token_sha256 = $tokenSha256
    lease_path = $lease.path
    lease_sha256 = $lease.sha256
    lease_identity = $lease.identity
    mutex_name = $MutexName
    write_probe_win32_error_code = $finalWriteProbeCode
  }
}

function Enter-BenchmarkLock {
  param(
    [string]$Context = "benchmark gate",
    [int]$TimeoutSeconds = 900,
    [string]$MutexName = $script:BenchmarkLockDefaultMutexName
  )

  if ([string]::IsNullOrWhiteSpace($Context)) {
    throw "Benchmark lock context must not be empty."
  }
  if ([string]::IsNullOrWhiteSpace($MutexName)) {
    throw "Benchmark lock mutex name must not be empty."
  }
  if ($TimeoutSeconds -lt 0) {
    throw "Benchmark lock timeout must not be negative."
  }

  if ($env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD -eq "1") {
    $previousEnvironment = Get-BenchmarkLockEnvironmentSnapshot
    $authority = Assert-BenchmarkLockInheritedAuthority `
      -Context $Context `
      -MutexName $MutexName

    Write-Host "Benchmark lock inherited for $Context from PID $($authority.owner_pid)."
    return [pscustomobject]@{
      acquired = $false
      inherited = $true
      abandoned = $false
      mutex = $null
      lease_stream = $null
      lease_path = $authority.lease_path
      lease_sha256 = $authority.lease_sha256
      lease_identity = $authority.lease_identity
      acquisition_id = $authority.acquisition_id
      owner_pid = $authority.owner_pid
      owner_started_at = $authority.owner_started_at
      context = $authority.context
      token_sha256 = $authority.token_sha256
      mutex_name = $authority.mutex_name
      write_probe_win32_error_code = $authority.write_probe_win32_error_code
      released = $false
      previous_environment = $previousEnvironment
      previous_held = $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD
      previous_owner_pid = $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID
      previous_context = $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT
      previous_token = $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN
    }
  }

  Initialize-BenchmarkLockNative
  $mutex = [System.Threading.Mutex]::new($false, $MutexName)
  $acquired = $false
  $abandoned = $false
  $leaseStream = $null
  $leasePath = $null
  $leaseIdentity = $null
  $previousEnvironment = Get-BenchmarkLockEnvironmentSnapshot
  try {
    Write-Host "Waiting for benchmark lock ($Context)..."
    try {
      $acquired = $mutex.WaitOne([TimeSpan]::FromSeconds($TimeoutSeconds))
    } catch [System.Threading.AbandonedMutexException] {
      $acquired = $true
      $abandoned = $true
      Write-Host "[WARN] Benchmark lock was abandoned by a prior process; continuing with recovered lock ($Context)."
    }
    if (-not $acquired) {
      throw "Timed out waiting for benchmark lock after ${TimeoutSeconds}s ($Context). Another benchmark/profiler gate may still be running."
    }

    $ownerStartedAt = Get-BenchmarkLockProcessStartIdentity -ProcessId $PID
    $token = [guid]::NewGuid().ToString("N")
    $tokenSha256 = Get-BenchmarkLockSha256Text -Value $token
    $acquisitionId = [guid]::NewGuid().ToString("N")
    $leaseDirectory = Get-BenchmarkLockLeaseDirectory
    $leasePath = Assert-BenchmarkLockLeasePath `
      -Path (Join-Path $leaseDirectory "active.json")

    $leaseStream = Open-BenchmarkLockActiveLease `
      -Path $leasePath `
      -TimeoutSeconds $TimeoutSeconds
    Assert-BenchmarkLockHandlePath `
      -Handle $leaseStream.SafeFileHandle `
      -ExpectedPath $leasePath
    $leaseIdentity = [VectorGrid.BenchmarkLockNative]::GetFileIdentity(
      $leaseStream.SafeFileHandle
    )
    $leaseAttributes = [VectorGrid.BenchmarkLockNative]::GetFileAttributes(
      $leaseStream.SafeFileHandle
    )
    if (
      ($leaseAttributes -band [int][System.IO.FileAttributes]::ReparsePoint) -ne 0
    ) {
      throw "Fixed canonical benchmark lease resolved to a reparse point."
    }
    $linkCount = [VectorGrid.BenchmarkLockNative]::GetNumberOfLinks(
      $leaseStream.SafeFileHandle
    )
    if ($linkCount -ne 1) {
      throw "Fixed canonical benchmark lease must have exactly one hard-link name."
    }
    $preWritePathIdentity = Get-BenchmarkLockLeasePathIdentity -Path $leasePath
    if ($preWritePathIdentity.identity -ne $leaseIdentity) {
      throw "Fixed canonical benchmark lease path does not identify the opened lease before publication."
    }

    $payload = [ordered]@{
      schema = $script:BenchmarkLockLeaseSchema
      acquisition_id = $acquisitionId
      created_at = (Get-Date).ToUniversalTime().ToString(
        "o",
        [System.Globalization.CultureInfo]::InvariantCulture
      )
      owner_pid = $PID
      owner_started_at = $ownerStartedAt
      context = $Context
      token_sha256 = $tokenSha256
      mutex_name = $MutexName
      released = $false
      file_identity = $leaseIdentity
    }
    $json = $payload | ConvertTo-Json -Compress
    $bytes = [System.Text.UTF8Encoding]::new($false).GetBytes($json)
    $leaseSha256 = Get-BenchmarkLockSha256Bytes -Bytes $bytes
    $leaseStream.SetLength(0)
    $leaseStream.Position = 0
    $leaseStream.Write($bytes, 0, $bytes.Length)
    $leaseStream.Flush($true)
    Assert-BenchmarkLockHandlePath `
      -Handle $leaseStream.SafeFileHandle `
      -ExpectedPath $leasePath

    $postWritePathIdentity = Get-BenchmarkLockLeasePathIdentity -Path $leasePath
    if ($postWritePathIdentity.identity -ne $leaseIdentity) {
      throw "Fixed canonical benchmark lease path changed identity during publication."
    }
    $stableLease = Read-BenchmarkLockLease -Path $leasePath
    if (
      $stableLease.identity -ne $leaseIdentity -or
      $stableLease.sha256 -ne $leaseSha256 -or
      "$($stableLease.payload.acquisition_id)" -ne $acquisitionId
    ) {
      throw "Fixed canonical benchmark lease changed identity or contents during publication."
    }
    Assert-BenchmarkLockLeaseHolder `
      -Path $leasePath `
      -OwnerPid $PID `
      -OwnerStartedAt $ownerStartedAt

    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD = "1"
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID = "$PID"
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_STARTED_AT = $ownerStartedAt
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT = $Context
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN = $token
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN_SHA256 = $tokenSha256
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_PATH = $leasePath
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_SHA256 = $leaseSha256
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_MUTEX_NAME = $MutexName

    Write-Host "Benchmark lock acquired ($Context)."
    return [pscustomobject]@{
      acquired = $true
      inherited = $false
      abandoned = $abandoned
      mutex = $mutex
      lease_stream = $leaseStream
      lease_path = $leasePath
      lease_sha256 = $leaseSha256
      lease_identity = $leaseIdentity
      acquisition_id = $acquisitionId
      owner_pid = $PID
      owner_started_at = $ownerStartedAt
      context = $Context
      token_sha256 = $tokenSha256
      mutex_name = $MutexName
      released = $false
      previous_environment = $previousEnvironment
      previous_held = $previousEnvironment[
        "TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD"
      ].value
      previous_owner_pid = $previousEnvironment[
        "TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID"
      ].value
      previous_context = $previousEnvironment[
        "TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT"
      ].value
      previous_token = $previousEnvironment[
        "TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN"
      ].value
    }
  } catch {
    $primaryError = $_
    if ($null -ne $leaseStream) {
      try {
        $leaseStream.Dispose()
      } catch {
        Write-Warning "Benchmark-lock partial-failure lease disposal also failed: $($_.Exception.Message)"
      }
    }
    try {
      Restore-BenchmarkLockEnvironment -Snapshot $previousEnvironment
    } catch {
      Write-Warning "Benchmark-lock partial-failure environment restoration also failed: $($_.Exception.Message)"
    }
    if ($acquired) {
      try {
        $mutex.ReleaseMutex() | Out-Null
      } catch {
        Write-Warning "Benchmark-lock partial-failure mutex release also failed: $($_.Exception.Message)"
      }
    }
    try {
      $mutex.Dispose()
    } catch {
      Write-Warning "Benchmark-lock partial-failure mutex disposal also failed: $($_.Exception.Message)"
    }
    throw $primaryError
  }
}

function Exit-BenchmarkLock {
  param($Handle)

  if ($null -eq $Handle) {
    return
  }
  if (-not $Handle.acquired) {
    if (
      [bool]$Handle.inherited -and
      ($null -eq $Handle.PSObject.Properties["released"] -or
        -not [bool]$Handle.released)
    ) {
      $authority = Assert-BenchmarkLockInheritedAuthority `
        -Context ([string]$Handle.context) `
        -MutexName ([string]$Handle.mutex_name)
      if (
        [int]$authority.owner_pid -ne [int]$Handle.owner_pid -or
        [string]$authority.owner_started_at -ne
        [string]$Handle.owner_started_at -or
        [string]$authority.lease_path -ne [string]$Handle.lease_path -or
        [string]$authority.lease_sha256 -ne [string]$Handle.lease_sha256 -or
        [string]$authority.lease_identity -ne [string]$Handle.lease_identity -or
        [string]$authority.acquisition_id -ne
        [string]$Handle.acquisition_id -or
        [string]$authority.token_sha256 -ne [string]$Handle.token_sha256 -or
        [string]$authority.mutex_name -ne [string]$Handle.mutex_name
      ) {
        throw "Benchmark lock inherited authority changed before child release."
      }
      if ($null -ne $Handle.PSObject.Properties["released"]) {
        $Handle.released = $true
      }
      Write-Host "Benchmark lock inheritance revalidated before child release."
    }
    return
  }
  if (
    $null -ne $Handle.PSObject.Properties["released"] -and
    [bool]$Handle.released
  ) {
    return
  }

  if ($null -ne $Handle.PSObject.Properties["released"]) {
    $Handle.released = $true
  }
  $releaseError = $null
  try {
    if ($null -ne $Handle.previous_environment) {
      Restore-BenchmarkLockEnvironment -Snapshot $Handle.previous_environment
    } else {
      Restore-BenchmarkLockEnvValue `
        -Name "TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD" `
        -Value $Handle.previous_held
      Restore-BenchmarkLockEnvValue `
        -Name "TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID" `
        -Value $Handle.previous_owner_pid
      Restore-BenchmarkLockEnvValue `
        -Name "TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT" `
        -Value $Handle.previous_context
      Restore-BenchmarkLockEnvValue `
        -Name "TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN" `
        -Value $Handle.previous_token
    }
  } catch {
    $releaseError = $_
  }

  try {
    if ($null -ne $Handle.lease_stream) {
      $Handle.lease_stream.Dispose()
      $Handle.lease_stream = $null
    }
  } catch {
    if ($null -eq $releaseError) {
      $releaseError = $_
    } else {
      Write-Warning "Benchmark-lock lease disposal also failed: $($_.Exception.Message)"
    }
  }

  try {
    $Handle.mutex.ReleaseMutex() | Out-Null
  } catch {
    if ($null -eq $releaseError) {
      $releaseError = $_
    } else {
      Write-Warning "Benchmark-lock mutex release also failed: $($_.Exception.Message)"
    }
  } finally {
    try {
      $Handle.mutex.Dispose()
    } catch {
    }
  }
  $Handle.acquired = $false
  Write-Host "Benchmark lock released."
  if ($null -ne $releaseError) {
    throw $releaseError
  }
}
