// Renders the app icon (1024×1024 PNG) with Core Graphics.
// Usage: swift Scripts/make-icon.swift <out.png>   (the original infinity icon: Resources/icon-infinity-1024.png)
import AppKit

let size: CGFloat = 1024
let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "icon.png"

let cs = CGColorSpace(name: CGColorSpace.sRGB)!
let ctx = CGContext(data: nil, width: Int(size), height: Int(size), bitsPerComponent: 8, bytesPerRow: 0,
                    space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!

func rgb(_ r: CGFloat, _ g: CGFloat, _ b: CGFloat, _ a: CGFloat = 1) -> CGColor {
    CGColor(colorSpace: cs, components: [r / 255, g / 255, b / 255, a])!
}

// macOS icon grid: 824pt rounded square centred on a 1024 canvas.
let tile = CGRect(x: 100, y: 100, width: 824, height: 824)
let tilePath = CGPath(roundedRect: tile, cornerWidth: 185, cornerHeight: 185, transform: nil)

// Drop shadow
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -12), blur: 28, color: rgb(0, 0, 0, 0.35))
ctx.addPath(tilePath)
ctx.setFillColor(rgb(20, 16, 40))
ctx.fillPath()
ctx.restoreGState()

// Background: deep night-sky gradient
ctx.saveGState()
ctx.addPath(tilePath)
ctx.clip()
let bg = CGGradient(colorsSpace: cs, colors: [rgb(58, 34, 110), rgb(22, 18, 52)] as CFArray, locations: [0, 1])!
ctx.drawLinearGradient(bg, start: CGPoint(x: 512, y: 924), end: CGPoint(x: 512, y: 100), options: [])
// Soft glow behind the emblem
let glow = CGGradient(colorsSpace: cs, colors: [rgb(255, 196, 92, 0.32), rgb(255, 196, 92, 0)] as CFArray, locations: [0, 1])!
ctx.drawRadialGradient(glow, startCenter: CGPoint(x: 512, y: 540), startRadius: 0,
                       endCenter: CGPoint(x: 512, y: 540), endRadius: 400, options: [])
// Scattered stars
let stars: [(CGFloat, CGFloat, CGFloat)] = [(230, 800, 7), (330, 860, 4), (780, 830, 6), (850, 720, 4),
                                            (190, 640, 4), (860, 330, 5), (250, 260, 4), (700, 210, 3), (420, 780, 3)]
for (x, y, r) in stars {
    ctx.setFillColor(rgb(255, 240, 210, 0.75))
    ctx.fillEllipse(in: CGRect(x: x - r, y: y - r, width: r * 2, height: r * 2))
}
ctx.restoreGState()

// Infinity loop ("Forever"): lemniscate of Bernoulli
func lemniscate(center c: CGPoint, a: CGFloat) -> CGPath {
    let p = CGMutablePath()
    let steps = 720
    for i in 0...steps {
        let t = CGFloat(i) / CGFloat(steps) * 2 * .pi
        let d = 1 + sin(t) * sin(t)
        let pt = CGPoint(x: c.x + a * cos(t) / d, y: c.y + a * sin(t) * cos(t) / d * 1.15)
        i == 0 ? p.move(to: pt) : p.addLine(to: pt)
    }
    p.closeSubpath()
    return p
}

let center = CGPoint(x: 512, y: 560)
let loop = lemniscate(center: center, a: 300)
let stroke = loop.copy(strokingWithWidth: 78, lineCap: .round, lineJoin: .round, miterLimit: 10)

// Gold gradient stroke with a dark outline for depth
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -10), blur: 18, color: rgb(0, 0, 0, 0.45))
ctx.addPath(stroke)
ctx.setFillColor(rgb(120, 70, 20))
ctx.fillPath()
ctx.restoreGState()

ctx.saveGState()
ctx.addPath(stroke)
ctx.clip()
let gold = CGGradient(colorsSpace: cs,
                      colors: [rgb(255, 236, 160), rgb(246, 190, 70), rgb(196, 122, 30)] as CFArray,
                      locations: [0, 0.55, 1])!
ctx.drawLinearGradient(gold, start: CGPoint(x: 512, y: 760), end: CGPoint(x: 512, y: 360), options: [])
ctx.restoreGState()

// Download arrow beneath the loop (the "manager" part)
let arrowColor = rgb(255, 226, 150)
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -6), blur: 10, color: rgb(0, 0, 0, 0.4))
ctx.setFillColor(arrowColor)
let shaft = CGPath(roundedRect: CGRect(x: 512 - 22, y: 250, width: 44, height: 120), cornerWidth: 22, cornerHeight: 22, transform: nil)
ctx.addPath(shaft)
ctx.fillPath()
let head = CGMutablePath()
head.move(to: CGPoint(x: 512 - 82, y: 290))
head.addLine(to: CGPoint(x: 512 + 82, y: 290))
head.addLine(to: CGPoint(x: 512, y: 196))
head.closeSubpath()
ctx.addPath(head.copy(strokingWithWidth: 18, lineCap: .round, lineJoin: .round, miterLimit: 10))
ctx.addPath(head)
ctx.fillPath()
ctx.restoreGState()

let image = ctx.makeImage()!
let rep = NSBitmapImageRep(cgImage: image)
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: out))
print("wrote \(out)")
