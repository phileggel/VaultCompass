import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { setDisplayLocale } from "@/lib/microUnits";
import commonEn from "./locales/en/common.json";
import commonFr from "./locales/fr/common.json";

const SUPPORTED_LANGS = ["fr", "en"];
const [browserLang = "en"] = navigator.language.split("-");
const detectedLng = SUPPORTED_LANGS.includes(browserLang) ? browserLang : "en";

i18n.use(initReactI18next).init({
  lng: detectedLng,
  fallbackLng: "en",
  defaultNS: "common",
  ns: ["common"],
  resources: {
    fr: { common: commonFr },
    en: { common: commonEn },
  },
  interpolation: {
    escapeValue: false,
  },
});

setDisplayLocale(i18n.language);
i18n.on("languageChanged", setDisplayLocale);

export default i18n;
