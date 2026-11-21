// Removes a "fake transparency" checkerboard that was baked into an image's pixels
// and writes a real RGBA PNG.
// Usage: swift Scripts/remove-checker.swift <in.png> <out.png>
//
// 1. Background = light, neutral (grey/white) pixels connected to the image border.
// 2. Edge pass: walking inward from the background, each pixel is "un-composited"
//    against the local checker colour (colour-to-alpha), so anti-aliased edges and
//    soft glows become semi-transparent instead of keeping a grey fringe. The walk
//    stops at pixels that are essentially opaque, so the artwork's interior is untouched.
import AppKit

let args = CommandLine.arguments
guard args.count == 3 else { print("usage: remove-checker <in.png> <out.png>"); exit(1) }

let src = NSImage(contentsOfFile: args[1])!.cgImage(forProposedRect: nil, context: nil, hints: nil)!
let w = src.width, h = src.height, n = w * h
let cs = CGColorSpace(name: CGColorSpace.sRGB)!
var px = [UInt8](repeating: 0, count: n * 4)
CGContext(data: &px, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4, space: cs,
          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    .draw(src, in: CGRect(x: 0, y: 0, width: w, height: h))

@inline(__always) func rgb(_ i: Int) -> (Double, Double, Double) {
    (Double(px[i * 4]), Double(px[i * 4 + 1]), Double(px[i * 4 + 2]))
}
func isCheckerLike(_ i: Int) -> Bool {
    let (r, g, b) = rgb(i)
    let mx = max(r, g, b), mn = min(r, g, b)
    return mn >= 196 && mx - mn <= 14
}

// 1. Flood fill the background from the border.
var bg = [Bool](repeating: false, count: n)
var queue: [Int] = []
func seed(_ i: Int) { if !bg[i] && isCheckerLike(i) { bg[i] = true; queue.append(i) } }
for x in 0..<w { seed(x); seed((h - 1) * w + x) }
for y in 0..<h { seed(y * w); seed(y * w + w - 1) }
var head = 0
while head < queue.count {
    let i = queue[head]; head += 1
    let x = i % w, y = i / w
    if x > 0 { seed(i - 1) }; if x < w - 1 { seed(i + 1) }
    if y > 0 { seed(i - w) }; if y < h - 1 { seed(i + w) }
}

// The checker's two shades (light and dark squares), measured from the background.
var lightSum = 0.0, lightN = 0.0, darkSum = 0.0, darkN = 0.0
for i in 0..<n where bg[i] {
    let v = rgb(i).0
    if v >= 236 { lightSum += v; lightN += 1 } else { darkSum += v; darkN += 1 }
}
// Pure white is included because compression noise pushes some light squares to 255.
let shades = [255, lightN > 0 ? lightSum / lightN : 252, darkN > 0 ? darkSum / darkN : 218]

// Colour-to-alpha against a neutral background value b: the minimum alpha that explains
// the observed colour as foreground-over-background.
func colorToAlpha(_ c: (Double, Double, Double), _ b: Double) -> (a: Double, r: Double, g: Double, bl: Double) {
    func chan(_ v: Double) -> Double { v < b ? (b - v) / b : (v > b ? (v - b) / (255 - b) : 0) }
    let a = min(1, max(chan(c.0), chan(c.1), chan(c.2)))
    guard a > 0.004 else { return (0, 0, 0, 0) }
    func un(_ v: Double) -> Double { min(255, max(0, (v - b) / a + b)) }
    return (a, un(c.0), un(c.1), un(c.2))
}

var alpha = [Double](repeating: 1, count: n)
for i in 0..<n where bg[i] { alpha[i] = 0 }

// 2. Edge / glow pass: BFS inward from the background through still-translucent pixels.
// Distance (in pixels, 4-connected) from the background, to find checker pockets
// trapped inside soft glows near the edge.
var dist = [Int](repeating: Int.max, count: n)
queue.removeAll(keepingCapacity: true)
for i in 0..<n where bg[i] { dist[i] = 0; queue.append(i) }
head = 0
while head < queue.count {
    let i = queue[head]; head += 1
    guard dist[i] < 12 else { continue }
    let x = i % w, y = i / w
    for j in [x > 0 ? i - 1 : -1, x < w - 1 ? i + 1 : -1, y > 0 ? i - w : -1, y < h - 1 ? i + w : -1]
    where j >= 0 && dist[j] == Int.max { dist[j] = dist[i] + 1; queue.append(j) }
}

var depth = [Int](repeating: -1, count: n)
queue.removeAll(keepingCapacity: true)
for i in 0..<n where !bg[i] {
    let x = i % w, y = i / w
    let touches = (x > 0 && bg[i - 1]) || (x < w - 1 && bg[i + 1]) || (y > 0 && bg[i - w]) || (y < h - 1 && bg[i + w])
    // Also seed checker-coloured pockets close to the edge (enclosed by glow).
    if touches || (dist[i] <= 10 && isCheckerLike(i)) { depth[i] = 0; queue.append(i) }
}
let maxDepth = 14
head = 0
while head < queue.count {
    let i = queue[head]; head += 1
    // Underneath could be a light or a dark checker square: take the more transparent explanation.
    let c = rgb(i)
    let res = shades.map { colorToAlpha(c, $0) }.min { $0.a < $1.a }!
    alpha[i] = res.a
    px[i * 4] = UInt8(res.r); px[i * 4 + 1] = UInt8(res.g); px[i * 4 + 2] = UInt8(res.bl)
    // Keep walking only while pixels are still noticeably see-through.
    guard res.a < 0.92, depth[i] < maxDepth else { continue }
    let x = i % w, y = i / w
    for j in [x > 0 ? i - 1 : -1, x < w - 1 ? i + 1 : -1, y > 0 ? i - w : -1, y < h - 1 ? i + w : -1]
    where j >= 0 && !bg[j] && depth[j] < 0 {
        depth[j] = depth[i] + 1
        queue.append(j)
    }
}

// 3. A faint glow over light vs dark squares gets slightly different alphas, leaving a
//    ghost checker. Average alpha over about one checker square, but only for the
//    translucent edge pixels, so solid artwork stays crisp.
var processed = [Bool](repeating: false, count: n)
for i in queue { processed[i] = true }
var smoothed = alpha
let r = 4
for i in 0..<n where processed[i] && alpha[i] < 0.9 {
    let x0 = i % w, y0 = i / w
    var sum = 0.0, cnt = 0.0
    for y in max(0, y0 - r)...min(h - 1, y0 + r) {
        for x in max(0, x0 - r)...min(w - 1, x0 + r) {
            let j = y * w + x
            if bg[j] || (processed[j] && alpha[j] < 0.9) { sum += alpha[j]; cnt += 1 }
        }
    }
    smoothed[i] = cnt > 0 ? sum / cnt : alpha[i]
}
alpha = smoothed

// Write premultiplied RGBA.
var out = [UInt8](repeating: 0, count: n * 4)
for i in 0..<n {
    let a = alpha[i]
    out[i * 4] = UInt8((Double(px[i * 4]) * a).rounded())
    out[i * 4 + 1] = UInt8((Double(px[i * 4 + 1]) * a).rounded())
    out[i * 4 + 2] = UInt8((Double(px[i * 4 + 2]) * a).rounded())
    out[i * 4 + 3] = UInt8((a * 255).rounded())
}
let outCtx = CGContext(data: &out, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4, space: cs,
                       bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
let rep = NSBitmapImageRep(cgImage: outCtx.makeImage()!)
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: args[2]))
let cleared = bg.filter { $0 }.count
print("\(args[2]): background \(cleared * 100 / n)%, edge pixels \(queue.count)")
