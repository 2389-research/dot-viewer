// ABOUTME: Standalone gutter view that displays line numbers alongside an NSScrollView.
// ABOUTME: Supports clickable line selection, current line highlighting, and dynamic gutter width.

import AppKit

/// A gutter view that draws line numbers aligned with the lines of an NSTextView.
///
/// This view is designed to sit beside the NSScrollView that contains the text view.
/// It observes the clip view's bounds changes to stay in sync with scrolling and
/// queries the text view's layout manager for line geometry.
///
/// ## Integration with EditorView
///
/// To integrate this gutter into an `NSViewRepresentable`, wrap the scroll view
/// and gutter in an `NSView` container:
///
/// ```swift
/// let container = NSView()
/// let gutter = LineNumberGutterView(textView: textView, scrollView: scrollView)
/// container.addSubview(gutter)
/// container.addSubview(scrollView)
/// // Use Auto Layout to pin gutter to the leading edge and scroll view beside it.
/// // gutter.leadingAnchor == container.leadingAnchor
/// // gutter.widthAnchor == gutter.requiredWidth
/// // scrollView.leadingAnchor == gutter.trailingAnchor
/// // scrollView.trailingAnchor == container.trailingAnchor
/// // Both top/bottom anchored to container.
/// ```
class LineNumberGutterView: NSView {

    override var isFlipped: Bool { true }

    // MARK: - Properties

    private weak var textView: NSTextView?
    private weak var scrollView: NSScrollView?

    private let font = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
    private let boldFont = NSFont.monospacedSystemFont(ofSize: 13, weight: .bold)
    private let gutterPadding: CGFloat = 8.0
    private let separatorWidth: CGFloat = 1.0

    /// Background tint for the gutter area, slightly different from the editor background.
    private let gutterBackgroundColor = NSColor.textBackgroundColor.blended(
        withFraction: 0.06, of: .separatorColor
    ) ?? NSColor.textBackgroundColor

    /// Color for the vertical separator line between gutter and editor.
    private let separatorColor = NSColor.separatorColor.withAlphaComponent(0.4)

    /// Normal line number color.
    private let lineNumberColor = NSColor.secondaryLabelColor

    /// Highlighted (current) line number color.
    private let currentLineNumberColor = NSColor.labelColor

    // MARK: - Initialization

    init(textView: NSTextView, scrollView: NSScrollView) {
        self.textView = textView
        self.scrollView = scrollView
        super.init(frame: .zero)
        self.translatesAutoresizingMaskIntoConstraints = false

        subscribeToScrollChanges()
        subscribeToTextChanges()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("LineNumberGutterView does not support NSCoder initialization.")
    }

    deinit {
        NotificationCenter.default.removeObserver(self)
    }

    // MARK: - Notification Subscriptions

    private func subscribeToScrollChanges() {
        guard let clipView = scrollView?.contentView else { return }
        clipView.postsBoundsChangedNotifications = true
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(clipViewBoundsDidChange(_:)),
            name: NSView.boundsDidChangeNotification,
            object: clipView
        )
    }

    private func subscribeToTextChanges() {
        guard let textView = textView else { return }
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleTextStorageDidProcessEditing(_:)),
            name: NSTextStorage.didProcessEditingNotification,
            object: textView.textStorage
        )
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(textViewSelectionDidChange(_:)),
            name: NSTextView.didChangeSelectionNotification,
            object: textView
        )
    }

    @objc private func clipViewBoundsDidChange(_ notification: Notification) {
        // Synchronous redraw to stay in lockstep with scroll position
        needsDisplay = true
        displayIfNeeded()
    }

    @objc private func handleTextStorageDidProcessEditing(_ notification: Notification) {
        invalidateIntrinsicContentSize()
        needsDisplay = true
    }

    @objc private func textViewSelectionDidChange(_ notification: Notification) {
        needsDisplay = true
    }

    // MARK: - Sizing

    /// Width needed to display the largest line number plus padding and separator.
    var requiredWidth: CGFloat {
        let lineCount = max(lineCountInTextView(), 1)
        let digitCount = String(lineCount).count
        let sampleString = String(repeating: "8", count: max(digitCount, 3)) as NSString
        let attributes: [NSAttributedString.Key: Any] = [.font: font]
        let textWidth = sampleString.size(withAttributes: attributes).width
        return ceil(textWidth + gutterPadding * 2 + separatorWidth)
    }

    override var intrinsicContentSize: NSSize {
        return NSSize(width: requiredWidth, height: NSView.noIntrinsicMetric)
    }

    // MARK: - Drawing

    override func draw(_ dirtyRect: NSRect) {
        guard let textView = textView,
              let layoutManager = textView.layoutManager,
              let textContainer = textView.textContainer,
              let scrollView = scrollView else { return }

        let visibleRect = scrollView.contentView.bounds
        let gutterRect = bounds

        // Draw gutter background
        gutterBackgroundColor.setFill()
        dirtyRect.fill()

        // Draw vertical separator
        separatorColor.setFill()
        let sepRect = NSRect(
            x: gutterRect.maxX - separatorWidth,
            y: dirtyRect.minY,
            width: separatorWidth,
            height: dirtyRect.height
        )
        sepRect.fill()

        // Determine the current line (line containing the insertion point)
        let selectedRange = textView.selectedRange()
        let nsText = textView.string as NSString
        let currentLineRange = nsText.lineRange(for: NSRange(location: selectedRange.location, length: 0))
        let currentLineNumber = lineNumber(forCharacterIndex: currentLineRange.location, in: nsText)

        // Walk visible glyphs and draw line numbers
        let textContainerOrigin = textView.textContainerOrigin
        let fullGlyphRange = layoutManager.glyphRange(for: textContainer)
        guard fullGlyphRange.length > 0 else {
            drawLineNumber(1, at: textContainerOrigin.y - visibleRect.origin.y, isCurrent: true)
            return
        }

        // Find the glyph range visible in the scroll view
        let visibleGlyphRange = layoutManager.glyphRange(
            forBoundingRect: visibleRect.offsetBy(dx: -textContainerOrigin.x, dy: -textContainerOrigin.y),
            in: textContainer
        )

        var previousLineNumber = -1
        var glyphIndex = visibleGlyphRange.location

        while glyphIndex < NSMaxRange(visibleGlyphRange) {
            let charIndex = layoutManager.characterIndexForGlyph(at: glyphIndex)
            let lineRange = nsText.lineRange(for: NSRange(location: charIndex, length: 0))
            let lineNum = lineNumber(forCharacterIndex: lineRange.location, in: nsText)

            if lineNum != previousLineNumber {
                let glyphRangeForLine = layoutManager.glyphRange(
                    forCharacterRange: lineRange,
                    actualCharacterRange: nil
                )
                var lineRect = layoutManager.lineFragmentRect(
                    forGlyphAt: glyphRangeForLine.location,
                    effectiveRange: nil
                )
                lineRect.origin.y += textContainerOrigin.y
                lineRect.origin.y -= visibleRect.origin.y

                let isCurrent = (lineNum == currentLineNumber)
                drawLineNumber(lineNum, at: lineRect.origin.y, isCurrent: isCurrent)
                previousLineNumber = lineNum
            }

            // Advance to the next line
            let effectiveGlyphRange = layoutManager.glyphRange(
                forCharacterRange: lineRange,
                actualCharacterRange: nil
            )
            glyphIndex = NSMaxRange(effectiveGlyphRange)
        }
    }

    /// Draws a single line number right-aligned in the gutter.
    private func drawLineNumber(_ number: Int, at yPosition: CGFloat, isCurrent: Bool) {
        let string = "\(number)" as NSString
        let chosenFont = isCurrent ? boldFont : font
        let chosenColor = isCurrent ? currentLineNumberColor : lineNumberColor
        let attributes: [NSAttributedString.Key: Any] = [
            .font: chosenFont,
            .foregroundColor: chosenColor,
        ]
        let size = string.size(withAttributes: attributes)
        let drawX = bounds.width - separatorWidth - gutterPadding - size.width
        let drawPoint = NSPoint(x: drawX, y: yPosition)
        string.draw(at: drawPoint, withAttributes: attributes)
    }

    // MARK: - Click Handling

    override func mouseDown(with event: NSEvent) {
        guard let textView = textView, let scrollView = scrollView else {
            super.mouseDown(with: event)
            return
        }

        let localPoint = convert(event.locationInWindow, from: nil)
        let visibleRect = scrollView.contentView.bounds
        let textContainerOrigin = textView.textContainerOrigin

        // Convert click y-coordinate to text view coordinate space
        let textY = localPoint.y + visibleRect.origin.y - textContainerOrigin.y

        guard let layoutManager = textView.layoutManager,
              let textContainer = textView.textContainer else { return }

        // Find the glyph at the click position
        let textPoint = NSPoint(x: 0, y: textY)
        let glyphIndex = layoutManager.glyphIndex(for: textPoint, in: textContainer)
        let charIndex = layoutManager.characterIndexForGlyph(at: glyphIndex)

        let nsText = textView.string as NSString
        guard charIndex < nsText.length else { return }

        let lineRange = nsText.lineRange(for: NSRange(location: charIndex, length: 0))
        textView.setSelectedRange(lineRange)
        textView.window?.makeFirstResponder(textView)
    }

    // MARK: - Helpers

    /// Returns the total number of lines in the text view.
    private func lineCountInTextView() -> Int {
        guard let textView = textView else { return 1 }
        let nsText = textView.string as NSString
        guard nsText.length > 0 else { return 1 }

        var lineCount = 0
        var index = 0
        while index < nsText.length {
            let lineRange = nsText.lineRange(for: NSRange(location: index, length: 0))
            lineCount += 1
            index = NSMaxRange(lineRange)
        }
        return lineCount
    }

    /// Returns the 1-based line number for a character index.
    private func lineNumber(forCharacterIndex targetIndex: Int, in nsText: NSString) -> Int {
        var lineNum = 1
        var index = 0
        while index < targetIndex && index < nsText.length {
            let lineRange = nsText.lineRange(for: NSRange(location: index, length: 0))
            if NSMaxRange(lineRange) <= targetIndex {
                lineNum += 1
            }
            index = NSMaxRange(lineRange)
        }
        return lineNum
    }
}
