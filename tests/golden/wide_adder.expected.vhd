library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_wide_adder is
  port (
    gl_p0_a : in unsigned(7 downto 0);
    gl_p1_b : in unsigned(7 downto 0);
    gl_p2_sum : out unsigned(8 downto 0)
  );
end entity gl_m0_wide_adder;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_wide_adder is
  signal gl_s2_sum : unsigned(8 downto 0);
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
    gl_s2_sum <= (resize(gl_p0_a, 9) + resize(gl_p1_b, 9));
  end process gl_comb_0;
  gl_p2_sum <= gl_s2_sum;
end architecture rtl;
