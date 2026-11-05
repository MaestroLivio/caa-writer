/** Stato di formattazione della selezione, condiviso fra editor e Ribbon. */
export interface FormattingState {
  bold: boolean;
  italic: boolean;
  underline: boolean;
  strike: boolean;
  highlight: boolean;
  color: string | null;
  fontSize: string | null;
  align: string | null;
  heading: number | null;
  indent: number;
  lineHeight: string | null;
  paragraphBorder: boolean;
  inTable: boolean;
  inList: boolean;
  canUndo: boolean;
  canRedo: boolean;
}

export const emptyFormatting: FormattingState = {
  bold: false,
  italic: false,
  underline: false,
  strike: false,
  highlight: false,
  color: null,
  fontSize: null,
  align: null,
  heading: null,
  indent: 0,
  lineHeight: null,
  paragraphBorder: false,
  inTable: false,
  inList: false,
  canUndo: false,
  canRedo: false,
};
