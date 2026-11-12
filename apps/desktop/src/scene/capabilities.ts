export function probeShaders(gl: WebGL2RenderingContext, fail: boolean) {
  const shaders: WebGLShader[] = [];
  const program = gl.createProgram();
  if (!program) throw new Error('Shader program 생성 실패');
  try {
    for (const [type, source] of [
      [gl.VERTEX_SHADER, '#version 300 es\nvoid main(){gl_Position=vec4(0.0,0.0,0.0,1.0);}'],
      [gl.FRAGMENT_SHADER, fail ? '#version 300 es\nintentional_invalid_shader' : '#version 300 es\nprecision highp float; out vec4 color; void main(){color=vec4(1.0);}'],
    ] as const) {
      const shader = gl.createShader(type);
      if (!shader) throw new Error('Shader 생성 실패');
      shaders.push(shader); gl.shaderSource(shader, source); gl.compileShader(shader);
      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(fail ? '진단용 Shader 실패 주입' : 'WebGL2 Shader compile 실패');
      gl.attachShader(program, shader);
    }
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error('WebGL2 Shader link 실패');
  } finally { gl.deleteProgram(program); shaders.forEach(shader => gl.deleteShader(shader)); }
}
