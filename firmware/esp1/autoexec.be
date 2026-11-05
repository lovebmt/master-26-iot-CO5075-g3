class Dht20
  var temp, hum, ticks
  def init()
    self.ticks = 0
  end
  def every_second()
    self.ticks = (self.ticks + 1) % 5
    if self.ticks != 0 return end
    self.temp = nil
    self.hum = nil
    wire1._begin_transmission(0x38)
    wire1._write(0xAC)
    wire1._write(0x33)
    wire1._write(0)
    wire1._end_transmission()
    tasmota.set_timer(100, / -> self.read_sample())
  end
  def read_sample()
    wire1._request_from(0x38, 7)
    var b = []
    while wire1._available() b.push(wire1._read()) end
    if size(b) != 7 || (b[0] & 0x80) != 0 return end
    var crc = 0xFF
    for i:0..5
      crc = crc ^ b[i]
      for j:0..7
        if crc & 0x80
          crc = ((crc << 1) ^ 0x31) & 0xFF
        else
          crc = (crc << 1) & 0xFF
        end
      end
    end
    if crc != b[6] return end
    self.hum = ((b[1] << 12) | (b[2] << 4) | (b[3] >> 4)) * 100.0 / 1048576
    self.temp = (((b[3] & 15) << 16) | (b[4] << 8) | b[5]) * 200.0 / 1048576 - 50.0
  end
  def json_append()
    if self.temp == nil return end
    tasmota.response_append(format(',\"DHT20\":{\"Temperature\":%.2f,\"Humidity\":%.2f}', self.temp, self.hum))
  end
end
tasmota.add_driver(Dht20())
