// Renders a 1024×1024 full-bleed app icon in a flat style: one white jigsaw piece centred on
// a solid colour (with a very subtle top-to-bottom shade). macOS rounds the corners itself.
// Usage: swift Scripts/make-flat-icon.swift <out.png> [background hex, default 31A8FF (Photoshop-style blue)]
import AppKit

let args = CommandLine.arguments
let out = args.count > 1 ? args[1] : "flat-icon.png"
let hex = UInt32(args.count > 2 ? args[2] : "31A8FF", radix: 16) ?? 0x31A8FF

let S = 1024.0
let cs = CGColorSpace(name: CGColorSpace.sRGB)!
func rgb(_ h: UInt32, lighten: Double = 0) -> CGColor {
    func ch(_ v: UInt32) -> Double { let c = Double(v & 0xff) / 255; return c + (lighten > 0 ? (1 - c) : c) * lighten }
    return CGColor(colorSpace: cs, components: [ch(h >> 16), ch(h >> 8), ch(h), 1])!
}
let ctx = CGContext(data: nil, width: Int(S), height: Int(S), bitsPerComponent: 8, bytesPerRow: 0,
                    space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!

// Background: the colour as a clear gradient, lighter at the top to darker at the bottom.
let bg = CGGradient(colorsSpace: cs, colors: [rgb(hex, lighten: 0.22), rgb(hex), rgb(hex, lighten: -0.42)] as CFArray,
                    locations: [0, 0.45, 1])!
ctx.drawLinearGradient(bg, start: CGPoint(x: 0, y: S), end: CGPoint(x: 0, y: 0), options: [])

// Jigsaw piece (same shape as the app's other pieces): knobs top/right, sockets left/bottom.
// Unit body square (0,0)–(1,1), y-up; knobs reach 0.27 beyond it.
let r = 0.17, k = 0.10, extent = 1 + k + r
let neck = asin(k / r)
let p = CGMutablePath()
p.move(to: CGPoint(x: 0, y: 0))
p.addLine(to: CGPoint(x: 0.5 - r * cos(neck), y: 0))
p.addArc(center: CGPoint(x: 0.5, y: k), radius: r, startAngle: .pi + neck, endAngle: -neck, clockwise: true)
p.addLine(to: CGPoint(x: 1, y: 0))
p.addLine(to: CGPoint(x: 1, y: 0.5 - r * cos(neck)))
p.addArc(center: CGPoint(x: 1 + k, y: 0.5), radius: r, startAngle: -.pi / 2 - neck, endAngle: .pi / 2 + neck, clockwise: false)
p.addLine(to: CGPoint(x: 1, y: 1))
p.addLine(to: CGPoint(x: 0.5 + r * cos(neck), y: 1))
p.addArc(center: CGPoint(x: 0.5, y: 1 + k), radius: r, startAngle: -neck, endAngle: .pi + neck, clockwise: false)
p.addLine(to: CGPoint(x: 0, y: 1))
p.addLine(to: CGPoint(x: 0, y: 0.5 + r * cos(neck)))
p.addArc(center: CGPoint(x: k, y: 0.5), radius: r, startAngle: .pi / 2 + neck, endAngle: -.pi / 2 - neck, clockwise: true)
p.closeSubpath()

// About 60% of the tile, like Discord's glyph. Centre the body square, nudged so the
// knobs (top/right) don't pull the piece off-centre visually.
let glyph = S * 0.60                     // full extent including knobs
let s = glyph / extent
let shift = (extent - 1) * s / 2         // half the knob overhang
var t = CGAffineTransform(translationX: S / 2 - s / 2 - shift * 0.5, y: S / 2 - s / 2 - shift * 0.5).scaledBy(x: s, y: s)
let piece = p.copy(using: &t)!

// Rounded corners on the body (flat-icon look) via a round-join stroke of the same colour.
// Drawn in a transparency layer so the shadow is cast once by the combined shape.
ctx.setShadow(offset: CGSize(width: 0, height: -18), blur: 36, color: CGColor(colorSpace: cs, components: [0, 0, 0.12, 0.45])!)
ctx.beginTransparencyLayer(auxiliaryInfo: nil)
ctx.setFillColor(CGColor(colorSpace: cs, components: [1, 1, 1, 1])!)
ctx.addPath(piece); ctx.fillPath()
ctx.addPath(piece.copy(strokingWithWidth: 14, lineCap: .round, lineJoin: .round, miterLimit: 10)); ctx.fillPath()
ctx.endTransparencyLayer()

try! NSBitmapImageRep(cgImage: ctx.makeImage()!).representation(using: .png, properties: [:])!
    .write(to: URL(fileURLWithPath: out))
print("wrote \(out)")
