// P1.1 vector illustration. No WebGL, telemetry, or runtime state ownership.
export function ReactorScene() {
  return <svg className="reactor-scene" viewBox="0 0 720 480" preserveAspectRatio="xMidYMid slice" fill="none" aria-hidden="true">
    <defs>
      <radialGradient id="reactor-haze">
        <stop stopColor="#65f9ee" stopOpacity=".3" />
        <stop offset=".36" stopColor="#208ba8" stopOpacity=".19" />
        <stop offset="1" stopColor="#07111f" stopOpacity="0" />
      </radialGradient>
      <radialGradient id="reactor-energy" cx=".43" cy=".36" r=".7">
        <stop stopColor="#effff6" /><stop offset=".1" stopColor="#abffec" />
        <stop offset=".24" stopColor="#41d9d3" /><stop offset=".47" stopColor="#146c89" />
        <stop offset=".75" stopColor="#092538" /><stop offset="1" stopColor="#030c16" />
      </radialGradient>
      <linearGradient id="reactor-orbit" x1="160" y1="100" x2="580" y2="360" gradientUnits="userSpaceOnUse">
        <stop stopColor="#234d69" /><stop offset=".35" stopColor="#8eeaf0" />
        <stop offset=".55" stopColor="#e3fff1" /><stop offset="1" stopColor="#266c94" stopOpacity=".15" />
      </linearGradient>
      <linearGradient id="reactor-ribbon" x1="290" y1="155" x2="425" y2="290" gradientUnits="userSpaceOnUse">
        <stop stopColor="#f0fff7" /><stop offset=".4" stopColor="#7cf7e0" stopOpacity=".8" />
        <stop offset="1" stopColor="#58a0ff" stopOpacity=".12" />
      </linearGradient>
      <radialGradient id="reactor-floor">
        <stop stopColor="#6edbe3" stopOpacity=".22" /><stop offset="1" stopColor="#102534" stopOpacity="0" />
      </radialGradient>
    </defs>

    {/* Distant chamber: perspective converges on the same origin as the Core. */}
    <ellipse cx="360" cy="238" rx="350" ry="236" fill="url(#reactor-haze)" />
    <g stroke="#3b647e" strokeOpacity=".3" strokeWidth=".7">
      <path d="M0 30 220 135M720 30 500 135M0 460 236 342M720 460 484 342" />
      <path d="M42 0 240 122M678 0 480 122M0 380 206 311M720 380 514 311" />
      <path d="M80 0V480M640 0V480M0 90H720M0 405H720" />
      <ellipse cx="360" cy="235" rx="302" ry="222" />
      <ellipse cx="360" cy="235" rx="280" ry="202" strokeDasharray="2 13" />
      <path d="M0 235H116M604 235H720M360 0V47M360 423V480" />
    </g>
    <g stroke="#7be4e7" strokeWidth="1" strokeOpacity=".65">
      <path d="M20 91H80V129M640 129V91H700M20 405H80V370M640 370V405H700" />
      <path d="M55 210V260M665 210V260M320 22H400M320 448H400" />
    </g>
    <g fill="#9fede6" opacity=".6">
      <path d="m80 89 3 3-3 3-3-3Zm560 0 3 3-3 3-3-3Z" />
      <circle cx="80" cy="405" r="1.5" /><circle cx="640" cy="405" r="1.5" />
    </g>

    {/* Large incomplete rear structure supplies scale without a wall of panels. */}
    <g className="reactor-structure" stroke="currentColor">
      <path d="M188 87A225 190 0 0 1 544 104M554 360A225 190 0 0 1 175 351" strokeOpacity=".4" strokeWidth="4" />
      <ellipse cx="360" cy="235" rx="221" ry="188" strokeOpacity=".32" />
      <ellipse cx="360" cy="235" rx="210" ry="179" strokeOpacity=".32" strokeWidth="6" strokeDasharray="1 12" />
      <path d="M138 235h18m408 0h18M360 47v13m0 350v13" strokeOpacity=".8" />
    </g>
    <ellipse cx="360" cy="393" rx="240" ry="63" fill="url(#reactor-floor)" />
    <g stroke="#51849f" strokeOpacity=".35">
      <ellipse cx="360" cy="393" rx="236" ry="46" />
      <ellipse cx="360" cy="393" rx="180" ry="28" />
      <path d="m180 420 61-16m298 16-61-16M360 438v-18" />
    </g>

    {/* Tilted orbital planes: fixed SVG perspective, not a 3D scene. */}
    <g stroke="url(#reactor-orbit)">
      <ellipse cx="360" cy="232" rx="215" ry="79" transform="rotate(-32 360 232)" strokeWidth="1.8" />
      <ellipse cx="360" cy="232" rx="201" ry="93" transform="rotate(48 360 232)" strokeWidth="1" />
      <ellipse cx="360" cy="232" rx="170" ry="155" strokeOpacity=".3" />
    </g>
    <g className="reactor-energy">
      <path d="M354 132C416 123 462 169 460 230C465 288 422 329 362 331C301 341 257 287 260 232C249 180 295 135 354 132Z" fill="url(#reactor-energy)" />
      <g stroke="url(#reactor-ribbon)">
        <path d="M290 272C265 224 291 155 355 149C423 139 459 216 425 274C403 312 352 319 317 298" strokeWidth="2.4" />
        <path d="M276 217C305 172 367 164 411 192C460 224 426 290 374 297C318 305 298 266 320 227" strokeWidth="1.4" />
        <path d="M333 137C288 193 324 259 385 268C449 272 454 213 424 177" strokeWidth="1.2" />
        <path d="M280 256C323 296 407 263 410 216C411 170 342 159 322 196C307 224 332 250 361 240" strokeOpacity=".65" />
        <path d="M303 166C369 128 442 194 424 252M309 298C275 262 287 213 311 197" strokeWidth="4" strokeOpacity=".19" />
      </g>
      <ellipse cx="343" cy="204" rx="47" ry="45" fill="url(#reactor-haze)" />
    </g>

    {/* Foreground arc occludes the energy to establish another depth plane. */}
    <path d="M157 288C220 350 437 330 549 230" stroke="#061421" strokeWidth="6" />
    <path d="M157 288C220 350 437 330 549 230" stroke="url(#reactor-orbit)" strokeWidth="1.7" />
    <g stroke="currentColor" strokeOpacity=".6" strokeWidth=".7">
      <path d="M8 173H92l38 22h35M712 291h-84l-38-22h-35" />
      <circle cx="165" cy="195" r="2" /><circle cx="555" cy="269" r="2" />
    </g>
  </svg>;
}
