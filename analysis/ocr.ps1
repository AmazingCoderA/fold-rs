Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetGenericArguments().Count -eq 1 -and $_.GetParameters().Count -eq 1 })[0]
function Await($winRtTask, $resultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($resultType)
    $netTask = $asTask.Invoke($null, @($winRtTask))
    $netTask.Wait(-1) | Out-Null
    $netTask.Result
}
[Windows.Storage.StorageFile,Windows.Storage,ContentType=WindowsRuntime] | Out-Null
[Windows.Graphics.Imaging.BitmapDecoder,Windows.Graphics.Imaging,ContentType=WindowsRuntime] | Out-Null
[Windows.Media.Ocr.OcrEngine,Windows.Foundation,ContentType=WindowsRuntime] | Out-Null

$path = "C:\Users\neon\Documents\ShareX\Screenshots\2026-11\QuickLook_K7rvnrxRMe.png"
$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($path)) ([Windows.Storage.StorageFile])
$stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])

$langs = [Windows.Media.Ocr.OcrEngine]::AvailableRecognizerLanguages
$out = New-Object System.Collections.Generic.List[string]
foreach ($l in $langs) { $out.Add("LANG $($l.LanguageTag)") }
$lang = New-Object Windows.Globalization.Language('ru')
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage($lang)
if ($null -eq $engine) { $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages() }
$result = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
foreach ($line in $result.Lines) {
    $box = $line.BoundingRect
    $out.Add(("[{0},{1} {2}x{3}] {4}" -f [int]$box.X, [int]$box.Y, [int]$box.Width, [int]$box.Height, $line.Text))
}
[System.IO.File]::WriteAllLines("D:\2026RustLev\RustFold\analysis\ocr_result.txt", $out, (New-Object System.Text.UTF8Encoding($false)))
