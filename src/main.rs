use rusb::UsbContext;
use rusb::{self, Context};

struct UsbInfo {
    ep_in: u8,
    ep_out: u8,
    interface_num: u8,
    usb_dev: Option<rusb::Device<Context>>,
    usb_handler: Option<rusb::DeviceHandle<Context>>,
}

impl UsbInfo {
    fn new() -> Self {
        UsbInfo {
            ep_in: 0,
            ep_out: 0,
            usb_handler: None,
            interface_num: 0,
            usb_dev: None,
        }
    }

    fn open(&mut self) -> bool {
        let context = rusb::Context::new().unwrap();

        let device_list = context.devices().unwrap();
        let num_device = device_list.len();

        println!("num device is: {}", num_device);

        for dev in device_list.iter() {
            let device_desc = dev.device_descriptor().unwrap();
            //过滤其他不是 CMSIS-DAP 设备
            match dev.open() {
                Ok(h) => {
                    let index = device_desc.product_string_index().unwrap();
                    if let Ok(s) = h.read_string_descriptor_ascii(index) {
                        if !s.ends_with("CMSIS-DAP") {
                            continue;
                        }
                        println!("{s}");
                        self.usb_handler = Some(h);
                    }
                }
                Err(_) => {
                    continue;
                }
            };

            for config in dev.config_descriptor(0).iter() {
                println!(
                    "vid: 0x{:x}, pid: 0x{:x}",
                    device_desc.vendor_id(),
                    device_desc.product_id()
                );

                for interface in config.interfaces().next().unwrap().descriptors() {
                    if interface.num_endpoints() < 2 {
                        continue;
                    }
                    self.interface_num = interface.interface_number();
                    let mut endpoint = interface.endpoint_descriptors().into_iter();
                    self.ep_in = endpoint.next().unwrap().address();
                    self.ep_out = endpoint.next().unwrap().address();

                    self.usb_dev = Some(dev);
                    return true;
                }
            }
        }
        false
    }

    fn write(&self, buf: &[u8]) -> (usize, String) {
        match self.usb_handler.as_ref().unwrap().write_bulk(self.ep_in, buf, std::time::Duration::from_secs(5)) {
            Ok(size) => (size, std::default::Default::default()),
            Err(e) => (0, e.to_string())
        }
    }

    fn read(&self, buf: &mut [u8]) -> (usize, String) {
        match self.usb_handler.as_ref().unwrap().read_bulk(self.ep_out, buf, std::time::Duration::from_secs(5)) {
            Ok(size) => (size, std::default::Default::default()),
            Err(e) => (0, e.to_string())
        }
    }
}

fn main() {
    let mut usb = UsbInfo::new();

    if usb.open() {
        println!("ep_in:0x{:x}, ep_out:0x{:x}", usb.ep_in, usb.ep_out);
        println!("设备打开成功");
    } else {
        println!("设备打开失败");
        std::process::exit(-1);
    }

    //声明接口
    match usb
        .usb_handler
        .as_ref()
        .unwrap()
        .claim_interface(usb.interface_num)
    {
        Ok(_) => println!("Successfully claimed interface {}.", usb.interface_num),
        Err(e) => {
            eprintln!("Failed to claim interface {}: {:?}", usb.interface_num, e);
            return;
        }
    }

    let mut buf = vec![0u8; 64];
    let mut rx = [0u8; 64];

    loop {
        let mut s = String::new();
        match std::io::stdin().read_line(&mut s) {
            Ok(_) => {
                let new: Vec<&str> = s.split_ascii_whitespace().collect();
                //转换成16进制
                for (i, hex_str) in new.iter().enumerate() {
                    // 将字符串转换为16进制
                    match u8::from_str_radix(hex_str, 16) {
                        Ok(val) => buf[i] = val,
                        Err(e) => {
                            println!("转换失败: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                println!("{e}");
                continue;
            }
        }

        let (mut n, mut res) = usb.write(&buf);
        if n > 0 {
            println!("Successfully wrote {} bytes.", n);
        } else {
            eprintln!("Failed to write data: {:?}", res);
        }

        (n, res) = usb.read(&mut rx);
        if n > 0 {
            println!("Successfully recv {} bytes.", n);
            println!("收到的数据: {:?}", &rx[0..n]);
        } else {
            eprintln!("Failed to write data: {:?}", res);
        }
    }
}
