// Static P1.1 scene composition. P1.2 replaces this renderer with real 3D.
export function ReactorScene() {
  return <svg className="reactor-scene" viewBox="0 0 720 480" preserveAspectRatio="xMidYMid slice" fill="none" aria-hidden="true">
    <defs>
      {/* Every illuminated surface refers to the same central source. */}
      <radialGradient id="reactor-light" gradientUnits="userSpaceOnUse" cx="360" cy="218" r="310">
        <stop stopColor="#d7fff1" /><stop offset=".32" stopColor="#70e8e0" />
        <stop offset=".68" stopColor="#285768" /><stop offset="1" stopColor="#142430" />
      </radialGradient>
      <radialGradient id="reactor-haze">
        <stop stopColor="#5af2dd" stopOpacity=".23" /><stop offset=".38" stopColor="#1b879a" stopOpacity=".13" />
        <stop offset="1" stopColor="#07111b" stopOpacity="0" />
      </radialGradient>
      <radialGradient id="reactor-energy" cx=".48" cy=".38" r=".62">
        <stop stopColor="#f0fff4" /><stop offset=".12" stopColor="#c2ffee" />
        <stop offset=".32" stopColor="#4bceca" stopOpacity=".85" />
        <stop offset=".67" stopColor="#12687b" stopOpacity=".45" /><stop offset="1" stopColor="#072535" stopOpacity="0" />
      </radialGradient>
      <radialGradient id="reactor-floor">
        <stop stopColor="#87eee5" stopOpacity=".2" /><stop offset=".48" stopColor="#3298a7" stopOpacity=".07" />
        <stop offset="1" stopColor="#0b1d2b" stopOpacity="0" />
      </radialGradient>
      <linearGradient id="reactor-edge" x1="164" y1="306" x2="548" y2="236" gradientUnits="userSpaceOnUse">
        <stop stopColor="#153244" /><stop offset=".45" stopColor="#aff5e5" /><stop offset="1" stopColor="#284556" />
      </linearGradient>
    </defs>

    {/* Background: quiet chamber, broad darkness, one rear aperture. */}
    <ellipse cx="360" cy="226" rx="340" ry="226" fill="url(#reactor-haze)" />
    <g stroke="#294351" strokeWidth=".8" strokeOpacity=".5">
      <path d="M0 15 196 137M720 15 524 137M0 470 207 337M720 470 513 337" />
      <path d="M56 0V480M664 0V480" />
      <ellipse cx="360" cy="224" rx="282" ry="208" />
    </g>
    <path d="M144 104A265 206 0 0 1 578 113" stroke="url(#reactor-light)" strokeWidth="9" strokeOpacity=".18" />
    <path d="M144 109A265 206 0 0 1 578 118" stroke="url(#reactor-light)" strokeWidth="1.2" strokeOpacity=".55" />
    <path d="M89 195v56m542-56v56M325 22h70" stroke="#466776" strokeWidth="1" />

    {/* Middle: two orbital planes; illumination falls away from the source. */}
    <g className="reactor-structure" stroke="url(#reactor-light)">
      <ellipse cx="360" cy="226" rx="218" ry="78" transform="rotate(-27 360 226)" strokeWidth="1.6" />
      <ellipse cx="360" cy="226" rx="185" ry="108" transform="rotate(56 360 226)" strokeWidth=".8" strokeOpacity=".6" />
      <path d="M170 164a214 174 0 0 1 44-54M521 324a214 174 0 0 1-43 33" strokeWidth="4" strokeDasharray="1 8" strokeOpacity=".65" />
    </g>
    <g className="reactor-energy">
      <path d="M326 120C370 144 430 127 456 188C474 233 414 252 420 300C358 343 283 296 271 242C255 191 290 164 326 120Z" fill="url(#reactor-energy)" />
      <g stroke="url(#reactor-light)" strokeLinecap="round">
        <path d="M303 282C273 239 314 156 370 155C424 153 439 227 405 267C376 300 333 291 322 261" strokeWidth="2" strokeOpacity=".8" />
        <path d="M286 221C314 176 394 179 420 226C437 261 381 296 340 272C302 249 329 206 361 207" strokeWidth="1" strokeOpacity=".55" />
        <path d="M333 151C305 190 331 245 382 252C423 259 433 215 416 189" strokeWidth="1.3" strokeOpacity=".8" />
      </g>
    </g>

    {/* Foreground: opaque sill and one source-lit edge; reflected light below. */}
    <ellipse cx="360" cy="391" rx="284" ry="73" fill="url(#reactor-floor)" />
    <path d="M96 352C204 414 518 414 624 352L684 480H36Z" fill="#060e18" fillOpacity=".8" />
    <path d="M96 352C204 414 518 414 624 352" stroke="url(#reactor-light)" strokeOpacity=".55" strokeWidth="1.5" />
    <ellipse cx="360" cy="400" rx="192" ry="42" fill="url(#reactor-floor)" />
    <path d="M164 306C257 344 454 312 548 236" stroke="#05121c" strokeWidth="7" />
    <path d="M164 303C257 341 454 309 548 233" stroke="url(#reactor-edge)" strokeWidth="1.8" />
    <g stroke="currentColor" strokeOpacity=".45" strokeWidth=".8">
      <path d="M18 269h66l43-28h19M702 269h-66l-43-28h-19" />
      <circle cx="146" cy="241" r="2" /><circle cx="574" cy="241" r="2" />
    </g>
  </svg>;
}
