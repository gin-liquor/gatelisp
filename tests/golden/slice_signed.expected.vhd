library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_slice_signed is
  port (
    gl_p0_input : in signed(15 downto 0);
    gl_p1_low : out unsigned(7 downto 0);
    gl_p2_high : out signed(7 downto 0)
  );
end entity gl_m0_slice_signed;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_slice_signed is
  signal gl_s1_low : unsigned(7 downto 0);
  signal gl_s2_high : signed(7 downto 0);
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
    gl_s1_low <= unsigned(gl_p0_input(((0 + 8) - 1) downto 0));
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s2_high <= signed(unsigned(gl_p0_input(((8 + 8) - 1) downto 8)));
  end process gl_comb_1;
  gl_p1_low <= gl_s1_low;
  gl_p2_high <= gl_s2_high;
end architecture rtl;
