interface BrandMarkProps {
  compact?: boolean
}

export function BrandMark({ compact = false }: BrandMarkProps) {
  return (
    <div className={compact ? 'brand-mark brand-mark--compact' : 'brand-mark'}>
      <span className="brand-mark__glyph" aria-hidden="true">
      </span>
      <strong>Koofr</strong>
    </div>
  )
}
