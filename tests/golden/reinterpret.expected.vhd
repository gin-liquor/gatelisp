library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_reinterpret_example is
  port (
    gl_p0_input : in unsigned(7 downto 0);
    gl_p1_signed_value : out signed(7 downto 0);
    gl_p2_round_trip : out unsigned(7 downto 0)
  );
end entity gl_m0_reinterpret_example;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_reinterpret_example is
  signal gl_s1_signed_value : signed(7 downto 0);
  signal gl_s2_round_trip : unsigned(7 downto 0);
  signal gl_s3_internal : signed(7 downto 0);
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
    gl_s3_internal <= signed(gl_p0_input);
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s1_signed_value <= gl_s3_internal;
  end process gl_comb_1;
  gl_comb_2 : process(all)
  begin
    gl_s2_round_trip <= unsigned(gl_s3_internal);
  end process gl_comb_2;
  gl_p1_signed_value <= gl_s1_signed_value;
  gl_p2_round_trip <= gl_s2_round_trip;
end architecture rtl;
