use windows::Win32::Media::Audio::{
    eRender, eMultimedia, IMMDeviceEnumerator, MMDeviceEnumerator,
    IAudioSessionManager2, IAudioSessionControl2, ISimpleAudioVolume,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED, CLSCTX_ALL, CoCreateInstance};
use windows::core::{Interface, GUID};

fn main() {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).unwrap();
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap();
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia).unwrap();
        let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None).unwrap();
        let session_enumerator = manager.GetSessionEnumerator().unwrap();
        let count = session_enumerator.GetCount().unwrap();
        println!("Sessions: {}", count);
    }
}
