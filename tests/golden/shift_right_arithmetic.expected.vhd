library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_shift_right_arithmetic_example is
  port (
    gl_p0_input : in signed(7 downto 0);
    gl_p1_value : out signed(7 downto 0)
  );
end entity gl_m0_shift_right_arithmetic_example;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_shift_right_arithmetic_example is
  signal gl_s1_value : signed(7 downto 0);
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
    gl_s1_value <= shift_right(gl_p0_input, 1);
  end process gl_comb_0;
  gl_p1_value <= gl_s1_value;
end architecture rtl;
