library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_and_gate is
  port (
    gl_p0_a : in std_logic;
    gl_p1_b : in std_logic;
    gl_p2_y : out std_logic
  );
end entity gl_m0_and_gate;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_and_gate is
  signal gl_s2_y : std_logic;
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_comb_0 : process(all)
  begin
    gl_s2_y <= (gl_p0_a and gl_p1_b);
  end process gl_comb_0;
  gl_p2_y <= gl_s2_y;
end architecture rtl;
