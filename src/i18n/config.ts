import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import commonEn from "./locales/en/common.json";
import commonFr from "./locales/fr/common.json";

i18n.use(initReactI18next).init({
  lng: "fr",
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

export default i18n;
