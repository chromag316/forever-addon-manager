// Renders a 1024×1024 full-bleed app icon: dark tile, gold ring with a jigsaw piece
// breaking out of it, a large gold infinity loop, and optionally two progress bars
// with green checks. macOS rounds the corners itself.
// Usage: swift Scripts/make-forever-icon.swift <out.png> [clean | bars]
import AppKit

let args = CommandLine.arguments
let out = args.count > 1 ? args[1] : "forever-icon.png"
let withBars = (args.count > 2 ? args[2] : "clean") == "bars"

let S = 1024.0
let cs = CGColorSpace(name: CGColorSpace.sRGB)!
func c(_ hex: UInt32, _ a: Double = 1) -> CGColor {
    CGColor(colorSpace: cs, components: [Double((hex >> 16) & 0xff) / 255, Double((hex >> 8) & 0xff) / 255,
                                         Double(hex & 0xff) / 255, a])!
}
let ctx = CGContext(data: nil, width: Int(S), height: Int(S), bitsPerComponent: 8, bytesPerRow: 0,
                    space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
let full = CGRect(x: 0, y: 0, width: S, height: S)

// Background: charcoal, slightly lighter in the middle.
let bg = CGGradient(colorsSpace: cs, colors: [c(0x2A2C33), c(0x0E0F12)] as CFArray, locations: [0, 1])!
ctx.drawRadialGradient(bg, startCenter: CGPoint(x: S / 2, y: S * 0.58), startRadius: 0,
                       endCenter: CGPoint(x: S / 2, y: S / 2), endRadius: S * 0.75, options: [.drawsAfterEndLocation])

let gold = CGGradient(colorsSpace: cs, colors: [c(0xFFF0B8), c(0xF7C54A), c(0xD98E1E), c(0x9E5C10)] as CFArray,
                      locations: [0, 0.35, 0.75, 1])!

/// Fill a shape with metallic gold: dark outline + drop shadow, gradient body, thin top highlight.
func drawGold(_ shape: CGPath, top: Double, bottom: Double, outline: Double = 10, shadow: Bool = true) {
    ctx.saveGState()
    if shadow { ctx.setShadow(offset: CGSize(width: 0, height: -10), blur: 22, color: c(0x000000, 0.6)) }
    ctx.addPath(shape.copy(strokingWithWidth: outline, lineCap: .round, lineJoin: .round, miterLimit: 10))
    ctx.addPath(shape)
    ctx.setFillColor(c(0x4A2A06))
    ctx.fillPath(using: .winding)
    ctx.restoreGState()

    ctx.saveGState()
    ctx.addPath(shape); ctx.clip()
    ctx.drawLinearGradient(gold, start: CGPoint(x: 0, y: top), end: CGPoint(x: 0, y: bottom),
                           options: [.drawsBeforeStartLocation, .drawsAfterEndLocation])
    ctx.restoreGState()
}

// Layout
let ringCenter = CGPoint(x: S / 2 - 8, y: withBars ? 600 : 500)
let ringR = withBars ? 300.0 : 405.0
let ringW = withBars ? 30.0 : 40.0
let pieceAngle = 45.0 * .pi / 180     // where the jigsaw piece breaks out of the ring

// Ring with a gap where the piece sits.
let gap = 0.30
let ring = CGMutablePath()
ring.addArc(center: ringCenter, radius: ringR, startAngle: pieceAngle + gap, endAngle: pieceAngle - gap + 2 * .pi, clockwise: false)
let ringShape = ring.copy(strokingWithWidth: ringW, lineCap: .round, lineJoin: .round, miterLimit: 10)
drawGold(ringShape, top: ringCenter.y + ringR, bottom: ringCenter.y - ringR, outline: 8)

// Jigsaw piece (knobs top and right, socket left/bottom), built as a path.
func jigsaw(center: CGPoint, body b: Double) -> CGPath {
    let r = b * 0.17, k = b * 0.10          // knob radius, knob centre offset outside the edge
    let x0 = center.x - b / 2, y0 = center.y - b / 2, x1 = x0 + b, y1 = y0 + b
    let cx = center.x, cy = center.y
    let p = CGMutablePath()
    let neck = asin(min(1, (k) / r))         // angle where the knob circle meets the edge
    p.move(to: CGPoint(x: x0, y: y0))
    // bottom edge with socket (inward)
    p.addLine(to: CGPoint(x: cx - r * cos(neck), y: y0))
    p.addArc(center: CGPoint(x: cx, y: y0 + k), radius: r, startAngle: .pi + neck, endAngle: -neck, clockwise: true)
    p.addLine(to: CGPoint(x: x1, y: y0))
    // right edge with knob (outward)
    p.addLine(to: CGPoint(x: x1, y: cy - r * cos(neck)))
    p.addArc(center: CGPoint(x: x1 + k, y: cy), radius: r, startAngle: -.pi / 2 - neck, endAngle: .pi / 2 + neck, clockwise: false)
    p.addLine(to: CGPoint(x: x1, y: y1))
    // top edge with knob (outward)
    p.addLine(to: CGPoint(x: cx + r * cos(neck), y: y1))
    p.addArc(center: CGPoint(x: cx, y: y1 + k), radius: r, startAngle: -neck, endAngle: .pi + neck, clockwise: false)
    p.addLine(to: CGPoint(x: x0, y: y1))
    // left edge with socket (inward)
    p.addLine(to: CGPoint(x: x0, y: cy + r * cos(neck)))
    p.addArc(center: CGPoint(x: x0 + k, y: cy), radius: r, startAngle: .pi / 2 + neck, endAngle: -.pi / 2 - neck, clockwise: true)
    p.closeSubpath()
    return p
}
let pieceCenter = CGPoint(x: ringCenter.x + ringR * cos(pieceAngle) + 10, y: ringCenter.y + ringR * sin(pieceAngle) + 6)
let pieceBody = withBars ? 150.0 : 180.0
var rot = CGAffineTransform(translationX: pieceCenter.x, y: pieceCenter.y).rotated(by: -0.12)
    .translatedBy(x: -pieceCenter.x, y: -pieceCenter.y)
let piece = jigsaw(center: pieceCenter, body: pieceBody).copy(using: &rot)!
drawGold(piece, top: pieceCenter.y + pieceBody, bottom: pieceCenter.y - pieceBody / 2, outline: 9)

// Infinity loop: lemniscate, with the rising strand drawn over the crossing for depth.
let infA = withBars ? 225.0 : 318.0
let infW = withBars ? 54.0 : 76.0
let infCenter = CGPoint(x: ringCenter.x, y: ringCenter.y - (withBars ? 10 : 15))
func lemniscatePoints(_ t0: Double, _ t1: Double, steps: Int = 400) -> CGPath {
    let p = CGMutablePath()
    for i in 0...steps {
        let t = t0 + (t1 - t0) * Double(i) / Double(steps)
        let d = 1 + sin(t) * sin(t)
        let pt = CGPoint(x: infCenter.x + infA * cos(t) / d, y: infCenter.y + infA * sin(t) * cos(t) / d * 1.3)
        i == 0 ? p.move(to: pt) : p.addLine(to: pt)
    }
    return p
}
let wholeLoop = lemniscatePoints(0, 2 * .pi, steps: 900)
let loopShape = wholeLoop.copy(strokingWithWidth: infW, lineCap: .round, lineJoin: .round, miterLimit: 10)
drawGold(loopShape, top: infCenter.y + infA * 0.45, bottom: infCenter.y - infA * 0.45)
// Over/under at the crossing: redraw the strand through the centre (around t = π/2) on top.
// Its dark edge lines only run along the sides; the gold part is a bit longer so it covers
// the ends seamlessly (same gradient as the loop underneath).
let edge = lemniscatePoints(.pi / 2 - 0.30, .pi / 2 + 0.30, steps: 120)
ctx.saveGState()
ctx.setShadow(offset: .zero, blur: 10, color: c(0x000000, 0.55))
ctx.addPath(edge.copy(strokingWithWidth: infW + 10, lineCap: .butt, lineJoin: .round, miterLimit: 10))
ctx.setFillColor(c(0x4A2A06)); ctx.fillPath()
ctx.restoreGState()
let over = lemniscatePoints(.pi / 2 - 0.40, .pi / 2 + 0.40, steps: 160)
ctx.saveGState()
ctx.addPath(over.copy(strokingWithWidth: infW, lineCap: .butt, lineJoin: .round, miterLimit: 10)); ctx.clip()
ctx.drawLinearGradient(gold, start: CGPoint(x: 0, y: infCenter.y + infA * 0.45), end: CGPoint(x: 0, y: infCenter.y - infA * 0.45),
                       options: [.drawsBeforeStartLocation, .drawsAfterEndLocation])
ctx.restoreGState()

// Progress bars with green checks.
if withBars {
    let check = CGGradient(colorsSpace: cs, colors: [c(0x8CF55A), c(0x34B81C)] as CFArray, locations: [0, 1])!
    for (i, fill) in [0.62, 0.44].enumerated() {
        let y = 215.0 - Double(i) * 105
        let bar = CGRect(x: 170, y: y, width: 520, height: 54)
        let barPath = CGPath(roundedRect: bar, cornerWidth: 27, cornerHeight: 27, transform: nil)
        ctx.saveGState()
        ctx.addPath(barPath); ctx.setFillColor(c(0x4A4D55)); ctx.fillPath()
        ctx.addPath(barPath); ctx.clip()
        let done = CGRect(x: bar.minX + bar.width * (1 - fill), y: y, width: bar.width * fill, height: 54)
        ctx.drawLinearGradient(check, start: CGPoint(x: 0, y: y + 54), end: CGPoint(x: 0, y: y), options: [])
        ctx.setFillColor(c(0x4A4D55)); ctx.fill(CGRect(x: bar.minX, y: y, width: bar.width * (1 - fill), height: 54))
        _ = done
        ctx.setStrokeColor(c(0xFFFFFF, 0.18)); ctx.setLineWidth(3)
        ctx.addPath(barPath); ctx.strokePath()
        ctx.restoreGState()
        // check mark
        let m = CGMutablePath()
        m.move(to: CGPoint(x: 740, y: y + 30))
        m.addLine(to: CGPoint(x: 775, y: y - 2))
        m.addLine(to: CGPoint(x: 845, y: y + 72))
        let mShape = m.copy(strokingWithWidth: 26, lineCap: .round, lineJoin: .round, miterLimit: 10)
        ctx.saveGState()
        ctx.setShadow(offset: CGSize(width: 0, height: -4), blur: 10, color: c(0x000000, 0.5))
        ctx.addPath(mShape); ctx.clip()
        ctx.drawLinearGradient(check, start: CGPoint(x: 0, y: y + 72), end: CGPoint(x: 0, y: y - 2), options: [])
        ctx.restoreGState()
    }
}

try! NSBitmapImageRep(cgImage: ctx.makeImage()!).representation(using: .png, properties: [:])!
    .write(to: URL(fileURLWithPath: out))
print("wrote \(out)")
