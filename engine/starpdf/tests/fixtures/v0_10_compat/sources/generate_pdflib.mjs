import fs from "node:fs";
import path from "node:path";
import {
  PDFArray,
  PDFDict,
  PDFDocument,
  PDFName,
  PDFNumber,
  PDFString,
  StandardFonts,
  rgb,
} from "pdf-lib";

const outputDirectory = process.argv[2];
if (!outputDirectory) {
  throw new Error("usage: node generate_pdflib.mjs OUTPUT_DIRECTORY");
}

async function baseDocument(title) {
  const document = await PDFDocument.create();
  document.setTitle(title);
  document.setCreator("StarPDF local pdf-lib compatibility generator");
  document.setProducer("pdf-lib 1.17.1");
  const page = document.addPage([612, 792]);
  const font = await document.embedFont(StandardFonts.Helvetica);
  page.drawText(title, { x: 48, y: 748, size: 16, font });
  return { document, page, font, form: document.getForm() };
}

async function save(document, name) {
  const bytes = await document.save({
    useObjectStreams: true,
    updateFieldAppearances: false,
  });
  fs.writeFileSync(path.join(outputDirectory, name), bytes);
}

{
  const { document, page, font, form } = await baseDocument(
    "pdf-lib complete producer form",
  );
  const shared = form.createTextField("shared.contact");
  shared.setText("Two producer widgets");
  shared.addToPage(page, { x: 48, y: 690, width: 230, height: 30, font });
  shared.addToPage(page, { x: 320, y: 690, width: 230, height: 30, font });
  shared.defaultUpdateAppearances(font);

  const notes = form.createTextField("notes");
  notes.enableMultiline();
  notes.setMaxLength(200);
  notes.setText("Producer multiline field");
  notes.addToPage(page, { x: 48, y: 590, width: 300, height: 75, font });
  notes.defaultUpdateAppearances(font);

  const checkbox = form.createCheckBox("accepted");
  checkbox.addToPage(page, { x: 48, y: 540, width: 24, height: 24 });
  checkbox.check();

  const radio = form.createRadioGroup("priority");
  radio.addOptionToPage("Low", page, { x: 100, y: 540, width: 24, height: 24 });
  radio.addOptionToPage("High", page, { x: 150, y: 540, width: 24, height: 24 });
  radio.select("High");

  const dropdown = form.createDropdown("country");
  dropdown.setOptions(["Malaysia", "Singapore", "Indonesia"]);
  dropdown.select("Malaysia");
  dropdown.addToPage(page, { x: 48, y: 480, width: 220, height: 32, font });
  dropdown.defaultUpdateAppearances(font);

  const list = form.createOptionList("languages");
  list.enableMultiselect();
  list.setOptions(["Bahasa Melayu", "English", "Tamil"]);
  list.select(["Bahasa Melayu", "English"]);
  list.addToPage(page, { x: 320, y: 450, width: 230, height: 90, font });
  list.defaultUpdateAppearances(font);
  await save(document, "pdflib-complete-form.pdf");
}

{
  const { document, page, font, form } = await baseDocument(
    "pdf-lib inherited field tree",
  );
  const field = form.createTextField("leaf");
  field.setText("Original leaf value");
  field.addToPage(page, { x: 48, y: 650, width: 300, height: 36, font });
  field.defaultUpdateAppearances(font);

  const child = field.acroField.dict;
  const childRef = field.acroField.ref;
  const parent = document.context.obj({
    T: PDFString.of("group"),
    FT: PDFName.of("Tx"),
    Ff: PDFNumber.of(0),
    DA: PDFString.of("/Helvetica 13 Tf 0 g"),
    Q: PDFNumber.of(2),
    MaxLen: PDFNumber.of(64),
    V: PDFString.of("Inherited current value"),
    DV: PDFString.of("Inherited default value"),
    Kids: [childRef],
  });
  const parentRef = document.context.register(parent);
  child.set(PDFName.of("Parent"), parentRef);
  for (const key of ["FT", "Ff", "DA", "Q", "MaxLen", "V", "DV"]) {
    child.delete(PDFName.of(key));
  }
  const fields = form.acroForm.dict.lookup(PDFName.of("Fields"), PDFArray);
  fields.set(0, parentRef);
  await save(document, "pdflib-inherited-field.pdf");
}

{
  const { document, page, font, form } = await baseDocument(
    "pdf-lib appearance preservation and NeedAppearances",
  );
  const field = form.createTextField("preserved.ap");
  field.setText("Original producer AP");
  field.addToPage(page, {
    x: 48,
    y: 650,
    width: 300,
    height: 36,
    font,
    borderColor: rgb(0.1, 0.3, 0.7),
    borderWidth: 2,
  });
  field.defaultUpdateAppearances(font);
  const widget = field.acroField.getWidgets()[0];
  if (!widget) throw new Error("expected a widget");
  const appearance = widget.dict.lookup(PDFName.of("AP"), PDFDict);
  const normal = appearance.get(PDFName.of("N"));
  if (!normal) throw new Error("expected a normal appearance");
  appearance.set(PDFName.of("R"), normal);
  appearance.set(PDFName.of("D"), normal);
  form.acroForm.dict.set(PDFName.of("NeedAppearances"), document.context.obj(true));
  await save(document, "pdflib-needappearances-ap-rd.pdf");
}
