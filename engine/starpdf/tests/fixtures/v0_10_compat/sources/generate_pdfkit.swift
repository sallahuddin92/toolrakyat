import AppKit
import Foundation
import PDFKit

guard CommandLine.arguments.count == 3 else {
    fputs("usage: generate_pdfkit.swift BASE_PDF OUTPUT_DIRECTORY\n", stderr)
    exit(2)
}

let baseURL = URL(fileURLWithPath: CommandLine.arguments[1])
let outputURL = URL(fileURLWithPath: CommandLine.arguments[2], isDirectory: true)

func document() -> PDFDocument {
    guard let result = PDFDocument(url: baseURL) else {
        fatalError("unable to open local base PDF")
    }
    result.documentAttributes = [
        PDFDocumentAttribute.producerAttribute: "Apple PDFKit 26.0",
        PDFDocumentAttribute.creatorAttribute: "StarPDF local compatibility generator",
        PDFDocumentAttribute.titleAttribute: "Redistributable StarPDF compatibility fixture",
    ]
    return result
}

func page(_ document: PDFDocument) -> PDFPage {
    guard let result = document.page(at: 0) else { fatalError("base PDF has no page") }
    return result
}

func write(_ document: PDFDocument, _ name: String) {
    let destination = outputURL.appendingPathComponent(name)
    guard document.write(to: destination) else { fatalError("unable to write \(name)") }
}

func border(_ width: CGFloat = 1) -> PDFBorder {
    let result = PDFBorder()
    result.lineWidth = width
    result.style = .solid
    return result
}

do {
    let pdf = document()
    let target = page(pdf)
    for (index, x) in [50.0, 330.0].enumerated() {
        let field = PDFAnnotation(
            bounds: CGRect(x: x, y: 650, width: 230, height: 34),
            forType: .widget,
            withProperties: nil
        )
        field.widgetFieldType = .text
        field.fieldName = "pdfkit.person.name"
        field.widgetStringValue = "PDFKit multi-widget \(index + 1)"
        field.widgetDefaultStringValue = "PDFKit default"
        field.font = NSFont.systemFont(ofSize: 13)
        field.fontColor = .black
        field.backgroundColor = .white
        field.color = .systemBlue
        field.border = border()
        target.addAnnotation(field)
    }

    let multiline = PDFAnnotation(
        bounds: CGRect(x: 50, y: 560, width: 300, height: 70),
        forType: .widget,
        withProperties: nil
    )
    multiline.widgetFieldType = .text
    multiline.fieldName = "pdfkit.notes"
    multiline.widgetStringValue = "Producer-authored multiline value"
    multiline.isMultiline = true
    multiline.maximumLength = 240
    multiline.font = NSFont.systemFont(ofSize: 12)
    multiline.backgroundColor = .white
    multiline.border = border()
    target.addAnnotation(multiline)

    let checkbox = PDFAnnotation(
        bounds: CGRect(x: 50, y: 510, width: 24, height: 24),
        forType: .widget,
        withProperties: nil
    )
    checkbox.widgetFieldType = .button
    checkbox.widgetControlType = .checkBoxControl
    checkbox.fieldName = "pdfkit.agree"
    checkbox.buttonWidgetStateString = "Accepted"
    checkbox.buttonWidgetState = .onState
    checkbox.border = border()
    target.addAnnotation(checkbox)
    write(pdf, "pdfkit-text-checkbox.pdf")
}

do {
    let pdf = document()
    let target = page(pdf)
    let choice = PDFAnnotation(
        bounds: CGRect(x: 50, y: 610, width: 250, height: 90),
        forType: .widget,
        withProperties: nil
    )
    choice.widgetFieldType = .choice
    choice.fieldName = "pdfkit.choice"
    choice.isListChoice = true
    choice.choices = ["Alpha", "Beta", "Gamma"]
    choice.values = ["A", "B", "C"]
    choice.widgetStringValue = "B"
    choice.font = NSFont.systemFont(ofSize: 12)
    choice.backgroundColor = .white
    choice.border = border()
    target.addAnnotation(choice)

    for (index, state) in ["First", "Second"].enumerated() {
        let radio = PDFAnnotation(
            bounds: CGRect(x: 50 + Double(index) * 60, y: 550, width: 24, height: 24),
            forType: .widget,
            withProperties: nil
        )
        radio.widgetFieldType = .button
        radio.widgetControlType = .radioButtonControl
        radio.fieldName = "pdfkit.radio"
        radio.buttonWidgetStateString = state
        radio.buttonWidgetState = index == 0 ? .onState : .offState
        radio.border = border()
        target.addAnnotation(radio)
    }
    write(pdf, "pdfkit-choice-radio.pdf")
}

do {
    let pdf = document()
    let target = page(pdf)
    let freeText = PDFAnnotation(
        bounds: CGRect(x: 50, y: 620, width: 260, height: 48),
        forType: .freeText,
        withProperties: nil
    )
    freeText.contents = "PDFKit FreeText producer appearance"
    freeText.font = NSFont.systemFont(ofSize: 13)
    freeText.fontColor = .black
    freeText.color = .systemGreen
    freeText.border = border(2)
    target.addAnnotation(freeText)

    for (index, type) in [PDFAnnotationSubtype.highlight, .underline, .strikeOut].enumerated() {
        let annotation = PDFAnnotation(
            bounds: CGRect(x: 50, y: 550 - Double(index) * 38, width: 260, height: 24),
            forType: type,
            withProperties: nil
        )
        annotation.contents = "PDFKit markup \(index)"
        annotation.color = index == 0 ? .yellow : .systemRed
        annotation.quadrilateralPoints = [
            NSValue(point: CGPoint(x: 0, y: 24)),
            NSValue(point: CGPoint(x: 260, y: 24)),
            NSValue(point: CGPoint(x: 0, y: 0)),
            NSValue(point: CGPoint(x: 260, y: 0)),
        ]
        target.addAnnotation(annotation)
    }
    write(pdf, "pdfkit-markup-freetext.pdf")
}

do {
    let pdf = document()
    let target = page(pdf)
    let square = PDFAnnotation(
        bounds: CGRect(x: 50, y: 600, width: 80, height: 60),
        forType: .square,
        withProperties: nil
    )
    square.color = .systemRed
    square.interiorColor = .systemPink
    square.border = border(3)
    target.addAnnotation(square)

    let circle = PDFAnnotation(
        bounds: CGRect(x: 160, y: 600, width: 80, height: 60),
        forType: .circle,
        withProperties: nil
    )
    circle.color = .systemBlue
    circle.interiorColor = .cyan
    circle.border = border(3)
    target.addAnnotation(circle)

    let line = PDFAnnotation(
        bounds: CGRect(x: 50, y: 520, width: 220, height: 50),
        forType: .line,
        withProperties: nil
    )
    line.startPoint = CGPoint(x: 5, y: 5)
    line.endPoint = CGPoint(x: 210, y: 42)
    line.startLineStyle = .openArrow
    line.endLineStyle = .closedArrow
    line.color = .systemIndigo
    line.border = border(2)
    line.contents = "PDFKit line"
    target.addAnnotation(line)

    let ink = PDFAnnotation(
        bounds: CGRect(x: 320, y: 520, width: 200, height: 140),
        forType: .ink,
        withProperties: nil
    )
    let path = NSBezierPath()
    path.move(to: CGPoint(x: 10, y: 20))
    path.curve(to: CGPoint(x: 180, y: 100), controlPoint1: CGPoint(x: 70, y: 130), controlPoint2: CGPoint(x: 120, y: 0))
    ink.add(path)
    ink.color = .systemPurple
    ink.border = border(2)
    target.addAnnotation(ink)

    let link = PDFAnnotation(
        bounds: CGRect(x: 50, y: 460, width: 220, height: 30),
        forType: .link,
        withProperties: nil
    )
    link.contents = "Local compatibility link"
    link.url = URL(string: "https://example.invalid/starpdf-v0.10")
    link.border = border()
    target.addAnnotation(link)
    write(pdf, "pdfkit-shapes-ink-link.pdf")
}

do {
    let pdf = document()
    let target = page(pdf)
    target.rotation = 90
    let field = PDFAnnotation(
        bounds: CGRect(x: 80, y: 560, width: 260, height: 38),
        forType: .widget,
        withProperties: nil
    )
    field.widgetFieldType = .text
    field.fieldName = "pdfkit.rotated"
    field.widgetStringValue = "Rotated producer field"
    field.font = NSFont.systemFont(ofSize: 13)
    field.backgroundColor = .white
    field.border = border(2)
    field.setValue(270, forAnnotationKey: .widgetRotation)
    target.addAnnotation(field)
    write(pdf, "pdfkit-rotated-widget.pdf")
}
