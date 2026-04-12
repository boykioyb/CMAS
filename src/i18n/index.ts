import { createI18n } from 'vue-i18n'
import vi from './vi'
import en from './en'

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  fallbackLocale: 'en',
  messages: {
    vi,
    en,
  },
})

export default i18n
