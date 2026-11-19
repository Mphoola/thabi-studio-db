import { useToasts } from "./useToasts.js";
import { toUserDetail } from "../lib/errors.js";
import {
  buildSvg,
  fileNameFor,
  resolvedColors,
  svgToPng,
} from "../lib/erSvg.js";
import { saveImage } from "../lib/tauri/commands.js";
import { pickImageDestination } from "../lib/tauri/files.js";

/**
 * Saves the diagram as SVG or PNG, through a save dialog, drawn from the same
 * boxes and lines as the screen.
 */
export function useDiagramExport({
  title,
  extent,
  nodes,
  lines,
  tables,
  keyed,
}) {
  const toasts = useToasts();

  return async function exportAs(format) {
    if (!extent.value.width) return;
    const svg = buildSvg(
      { ...extent.value, nodes: nodes.value, lines: lines.value },
      tables.value,
      resolvedColors(),
      keyed.value,
    );
    try {
      const path = await pickImageDestination(
        fileNameFor(title(), format),
        format,
      );
      if (!path) return;
      const saved = await saveImage(
        path,
        format === "png"
          ? {
              pngBase64: await svgToPng(
                svg,
                extent.value.width,
                extent.value.height,
              ),
            }
          : { svg },
      );
      toasts.success("Diagram saved", saved);
    } catch (error) {
      toasts.error("Couldn't save the diagram", toUserDetail(error));
    }
  };
}
