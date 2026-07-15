import { Check } from 'lucide-react'
import type { CSSProperties } from 'react'
import { themes, type ThemeId } from '../theme'

interface ThemePickerProps {
  value: ThemeId
  onChange: (theme: ThemeId) => void
}

export function ThemePicker({ value, onChange }: ThemePickerProps) {
  return (
    <div className="theme-picker">
      <p className="theme-picker__intro">选择应用的强调色。设置只保存在这台设备上。</p>
      <div className="theme-picker__grid" role="radiogroup" aria-label="颜色主题">
        {themes.map((theme) => {
          const selected = theme.id === value
          const style = { '--theme-swatch': theme.accent } as CSSProperties

          return (
            <button
              className={`theme-option${selected ? ' theme-option--selected' : ''}`}
              key={theme.id}
              type="button"
              role="radio"
              aria-checked={selected}
              onClick={() => onChange(theme.id)}
              style={style}
            >
              <span className="theme-option__swatch" aria-hidden="true" />
              <span className="theme-option__copy">
                <strong>{theme.label}</strong>
                <small>{theme.description}</small>
              </span>
              <span className="theme-option__check" aria-hidden="true">
                {selected ? <Check size={15} strokeWidth={2.4} /> : null}
              </span>
            </button>
          )
        })}
      </div>
    </div>
  )
}
