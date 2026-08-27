import { useEffect, useMemo, useState, type Ref } from "react";

import { listInstalledFontFamilies } from "../fontBridge";
import { systemFontFamily, useUiPreferences } from "../uiPreferences";

export function PreferencesPanel({ onClose, panelRef }: { onClose: () => void; panelRef: Ref<HTMLElement> }) {
  const { chatFontScale, fontFamily, locale, setChatFontScale, setFontFamily, setLocale, t } = useUiPreferences();
  const [installedFonts, setInstalledFonts] = useState<string[] | null>(null);
  useEffect(() => {
    let isCurrent = true;
    void listInstalledFontFamilies().then((families) => { if (isCurrent) setInstalledFonts(families); });
    return () => { isCurrent = false; };
  }, []);
  const fontOptions = useMemo(
    () => fontFamily === systemFontFamily || installedFonts?.includes(fontFamily)
      ? installedFonts ?? []
      : [fontFamily, ...(installedFonts ?? [])],
    [fontFamily, installedFonts],
  );
  return (
    <section aria-label={t("preferences")} className="preferences-panel" ref={panelRef} role="dialog">
      <header className="popover-heading">
        <div><strong>{t("preferencesTitle")}</strong><span>{t("preferencesSubtitle")}</span></div>
        <button onClick={onClose} title={t("close")} type="button"><span aria-hidden="true">×</span><span className="sr-only">{t("close")}</span></button>
      </header>
      <label className="preference-field">
        <span>{t("language")}</span>
        <select onChange={(event) => setLocale(event.target.value === "en" ? "en" : "ja")} value={locale}>
          <option value="ja">{t("japanese")}</option><option value="en">{t("english")}</option>
        </select>
      </label>
      <label className="preference-field">
        <span>{t("fontFamily")}</span>
        <select className="font-family-select" onChange={(event) => setFontFamily(event.target.value)} value={fontFamily}>
          <option value={systemFontFamily}>{t("systemFont")}</option>
          {fontOptions.map((family) => (
            <option key={family} style={{ fontFamily: family }} value={family}>{family}</option>
          ))}
        </select>
        {installedFonts === null ? <small>{t("installedFontsLoading")}</small> : null}
        {installedFonts?.length === 0 ? <small>{t("noInstalledFonts")}</small> : null}
      </label>
      <label className="preference-field">
        <span>{t("fontSize")} <output>{t("fontSizeValue", { value: Math.round(chatFontScale * 100) })}</output></span>
        <input max="1.5" min="0.8" onChange={(event) => setChatFontScale(Number(event.target.value))} step="0.05" type="range" value={chatFontScale} />
      </label>
      <p className="preference-note">{t("preferenceSaved")}</p>
    </section>
  );
}
