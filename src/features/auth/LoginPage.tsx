import { useState, type FormEvent } from 'react'
import {
  Cloud,
  Eye,
  EyeOff,
  FileText,
  LockKeyhole,
  Monitor,
  ShieldCheck,
} from 'lucide-react'
import { BrandMark } from '../../components/BrandMark'

interface LoginPageProps {
  busy: boolean
  error: string
  onLogin: (email: string, appPassword: string) => Promise<void>
  onThemeClick: () => void
}

export function LoginPage({ busy, error, onLogin, onThemeClick }: LoginPageProps) {
  const [email, setEmail] = useState('')
  const [appPassword, setAppPassword] = useState('')
  const [showPassword, setShowPassword] = useState(false)
  const [validationError, setValidationError] = useState('')

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const normalizedEmail = email.trim()

    if (!normalizedEmail || !normalizedEmail.includes('@')) {
      setValidationError('请输入有效的邮箱地址。')
      return
    }
    if (!appPassword) {
      setValidationError('请输入应用专用密码。')
      return
    }

    setValidationError('')
    await onLogin(normalizedEmail, appPassword)
    setAppPassword('')
  }

  const visibleError = validationError || error

  return (
    <main className="auth-layout">
      <section className="auth-story" aria-label="Koofr 安全连接说明">
        <div className="auth-story__brand">
          <BrandMark />
        </div>

        <div className="auth-illustration" aria-hidden="true">
          <div className="auth-file auth-file--left"><FileText /></div>
          <div className="auth-file auth-file--center"><FileText /></div>
          <div className="auth-file auth-file--right"><FileText /></div>
          <div className="auth-flow auth-flow--left" />
          <div className="auth-flow auth-flow--right" />
          <div className="auth-flow auth-flow--down" />
          <span className="auth-flow__check">✓</span>
          <div className="auth-cloud">
            <Cloud />
            <LockKeyhole />
          </div>
        </div>

        <div className="auth-privacy">
          <ShieldCheck aria-hidden="true" />
          <p>你的登录信息只用于建立本次连接，<br />不会保存在网页存储中。</p>
        </div>
      </section>

      <section className="auth-panel">
        <form className="auth-form" autoComplete="off" noValidate onSubmit={submit}>
          <header className="auth-form__heading">
            <h1>登录 Koofr</h1>
            <p>使用邮箱和应用专用密码连接你的云端文件。</p>
          </header>

          <div className="auth-field">
            <label htmlFor="koofr-email">邮箱地址</label>
            <input
              id="koofr-email"
              type="email"
              name="koofr-email"
              value={email}
              placeholder="name@example.com"
              autoCapitalize="none"
              autoCorrect="off"
              spellCheck={false}
              disabled={busy}
              aria-invalid={Boolean(visibleError)}
              onChange={(event) => {
                setEmail(event.target.value)
                setValidationError('')
              }}
            />
          </div>

          <div className="auth-field">
            <label htmlFor="koofr-app-password">应用专用密码</label>
            <span className="auth-password">
              <input
                id="koofr-app-password"
                type={showPassword ? 'text' : 'password'}
                name="koofr-app-password"
                value={appPassword}
                placeholder="输入应用专用密码"
                disabled={busy}
                aria-invalid={Boolean(visibleError)}
                onChange={(event) => {
                  setAppPassword(event.target.value)
                  setValidationError('')
                }}
              />
              <button
                type="button"
                aria-label={showPassword ? '隐藏密码' : '显示密码'}
                aria-pressed={showPassword}
                disabled={busy}
                onClick={() => setShowPassword((visible) => !visible)}
              >
                {showPassword ? <EyeOff /> : <Eye />}
              </button>
            </span>
          </div>

          <div className={`auth-error${visibleError ? ' auth-error--visible' : ''}`} role="alert">
            {visibleError}
          </div>

          <button className="auth-submit" type="submit" disabled={busy}>
            {busy ? <><span className="auth-spinner" />正在连接…</> : '登录'}
          </button>

          <p className="auth-hint">
            <LockKeyhole aria-hidden="true" />
            请使用 Koofr 设置中生成的应用专用密码。
          </p>
        </form>

        <button className="auth-theme" type="button" onClick={onThemeClick}>
          <Monitor aria-hidden="true" />
          外观
        </button>
      </section>
    </main>
  )
}
